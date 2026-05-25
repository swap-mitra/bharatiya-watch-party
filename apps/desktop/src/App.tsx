import { Component, type ErrorInfo, type ReactNode, useEffect, useMemo, useRef, useState } from 'react';

import { buildHostHeartbeat, HOST_HEARTBEAT_INTERVAL_MS, resolveHeartbeatPlan } from './lib/playbackSync';
import { connectRoomSocket, createRoom, joinRoom, sendEnvelope } from './lib/roomClient';
import { onPlayerState, onPlayerTracks, registerWebPlayerElement, tauriPlayer } from './lib/tauri';
import type {
  ChatMessage,
  CreateRoomResponse,
  JoinRoomResponse,
  PlaybackAction,
  PlaybackCommand,
  PlaybackHeartbeat,
  PlayerState,
  RoomCloseReason,
  RoomSession,
  RoomSnapshot,
  RoomSurfaceState,
  ServerEnvelope,
  TrackCatalog,
  TransportState,
} from './lib/types';
import './styles/app.css';

const initialPlayerState: PlayerState = {
  status: 'idle',
  activeSource: null,
  positionMs: 0,
  durationMs: null,
  volume: 100,
  muted: false,
  playbackRatePercent: 100,
  selectedAudioTrack: null,
  selectedSubtitleTrack: null,
  lastError: null,
};

const initialTracks: TrackCatalog = { audio: [], subtitles: [] };
const MAX_RECONNECT_ATTEMPTS = 5;
const RECONNECT_BASE_DELAY_MS = 1500;
const CHAT_HISTORY_LIMIT = 50;

export default function App() {
  return (
    <AppErrorBoundary>
      <WatchPartyApp />
    </AppErrorBoundary>
  );
}

function WatchPartyApp() {
  const socketRef = useRef<WebSocket | null>(null);
  const reconnectTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const heartbeatTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const copiedTimerRef = useRef<ReturnType<typeof setTimeout> | null>(null);
  const manualCloseRef = useRef(false);
  const nextSeqRef = useRef(1);
  const roomSessionRef = useRef<RoomSession | null>(null);
  const roomClosedReasonRef = useRef<RoomCloseReason | null>(null);
  const playerStateRef = useRef<PlayerState>(initialPlayerState);
  const sendHostHeartbeatRef = useRef<() => void>(() => undefined);
  const lastRoomCommandSeqRef = useRef(0);
  const lastSyncCorrectionAtRef = useRef(0);
  const videoRef = useRef<HTMLVideoElement | null>(null);

  const [playerState, setPlayerState] = useState<PlayerState>(initialPlayerState);
  const [tracks, setTracks] = useState<TrackCatalog>(initialTracks);
  const [mode, setMode] = useState<'standard' | 'theater'>('standard');
  const [roomSession, setRoomSession] = useState<RoomSession | null>(null);
  const [roomSnapshot, setRoomSnapshot] = useState<RoomSnapshot | null>(null);
  const [transportState, setTransportState] = useState<TransportState>('idle');
  const [roomClosedReason, setRoomClosedReason] = useState<RoomCloseReason | null>(null);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [streamUrl, setStreamUrl] = useState(
    'https://commondatastorage.googleapis.com/gtv-videos-bucket/sample/BigBuckBunny.mp4',
  );
  const [seekValue, setSeekValue] = useState(120000);
  const [chatDraft, setChatDraft] = useState('');
  const [createName, setCreateName] = useState('Host');
  const [joinName, setJoinName] = useState('Viewer');
  const [joinCode, setJoinCode] = useState('');
  const [roomError, setRoomError] = useState<string | null>(null);
  const [playerBackend, setPlayerBackend] = useState('mock');
  const [playerWarning, setPlayerWarning] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [selfReady, setSelfReady] = useState(false);
  const [copyState, setCopyState] = useState<'idle' | 'copied' | 'failed'>('idle');

  useEffect(() => {
    roomSessionRef.current = roomSession;
  }, [roomSession]);

  useEffect(() => {
    playerStateRef.current = playerState;
  }, [playerState]);

  useEffect(() => {
    sendHostHeartbeatRef.current = () => {
      const heartbeat = buildHostHeartbeat({
        session: roomSessionRef.current,
        state: playerStateRef.current,
        commandSeq: Math.max(0, nextSeqRef.current - 1),
        nowMs: Date.now(),
      });
      if (!heartbeat) {
        return;
      }

      try {
        sendEnvelope(socketRef.current, {
          type: 'playback_heartbeat',
          payload: heartbeat,
        });
      } catch (error) {
        pushEvent(`Heartbeat skipped: ${String(error)}`);
      }
    };
  });

  useEffect(() => {
    roomClosedReasonRef.current = roomClosedReason;
  }, [roomClosedReason]);

  useEffect(() => {
    clearHeartbeatTimer();
    if (!roomSession || roomSession.role !== 'host' || transportState !== 'connected' || roomClosedReason) {
      return;
    }

    heartbeatTimerRef.current = setInterval(() => {
      sendHostHeartbeatRef.current();
    }, HOST_HEARTBEAT_INTERVAL_MS);

    return clearHeartbeatTimer;
  }, [roomSession, transportState, roomClosedReason]);

  useEffect(() => {
    registerWebPlayerElement(playerBackend === 'web-video' ? videoRef.current : null);
    return () => registerWebPlayerElement(null);
  }, [playerBackend, roomSession]);

  useEffect(() => {
    tauriPlayer
      .bootstrap()
      .then(({ state, tracks: initialCatalog, backend, backendWarning }) => {
        setPlayerState(state);
        setTracks(initialCatalog);
        setPlayerBackend(backend);
        setPlayerWarning(backendWarning ?? null);
      })
      .catch((error: unknown) => {
        pushEvent(`Bootstrap failed: ${String(error)}`);
      });

    const unlisten = Promise.all([
      onPlayerState((state) => {
        setPlayerState(state);
        pushEvent(`State -> ${state.status} @ ${state.positionMs}ms`);
      }),
      onPlayerTracks((nextTracks) => {
        setTracks(nextTracks);
        pushEvent('Track catalog updated');
      }),
    ]);

    return () => {
      clearReconnectTimer();
      clearHeartbeatTimer();
      clearCopiedTimer();
      if (socketRef.current) {
        manualCloseRef.current = true;
        socketRef.current.close();
        socketRef.current = null;
      }
      void unlisten.then((handlers) => handlers.forEach((handler) => handler()));
    };
  }, []);

  const isHost = roomSession?.role === 'host';
  const participantCount = roomSnapshot?.participants.length ?? 0;
  const readyCount = roomSnapshot?.participants.filter((participant) => participant.ready).length ?? 0;
  const viewerCount = roomSnapshot?.participants.filter((participant) => participant.role === 'viewer').length ?? 0;
  const roomInteractive = transportState === 'connected' && roomClosedReason === null;
  const canOperatePlayer = !roomSession || (isHost && roomInteractive);
  const canSendChat = roomInteractive;
  const createNameError = getDisplayNameError(createName);
  const joinNameError = getDisplayNameError(joinName);
  const joinCodeError = getRoomCodeError(joinCode);
  const chatLength = chatDraft.trim().length;
  const chatTooLong = chatLength > 500;
  const subtitleValue = playerState.selectedSubtitleTrack ?? 'none';
  const statusLabel = useMemo(
    () => playerState.status.replace(/(^\w)/, (letter) => letter.toUpperCase()),
    [playerState.status],
  );
  const surfaceState = deriveSurfaceState(roomSession, transportState, roomClosedReason, playerState.status);
  const roomErrorTitle = roomError ? deriveRoomErrorTitle(roomError) : null;
  const readyPercent = participantCount > 0 ? Math.round((readyCount / participantCount) * 100) : 0;

  async function handleCreateRoom() {
    if (createNameError) {
      setRoomError(createNameError);
      return;
    }

    setIsSubmitting(true);
    setRoomError(null);
    try {
      const response = await createRoom(createName.trim());
      pushEvent(`Room ${response.roomCode} created`);
      openSocket(buildRoomSession(response));
    } catch (error) {
      setRoomError(String(error));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleJoinRoom() {
    if (joinCodeError || joinNameError) {
      setRoomError(joinCodeError ?? joinNameError ?? 'Enter a valid room code and display name.');
      return;
    }

    setIsSubmitting(true);
    setRoomError(null);
    try {
      const response = await joinRoom(joinCode.trim().toUpperCase(), joinName.trim());
      pushEvent(`Joining room ${response.roomCode}`);
      openSocket(buildRoomSession(response), { initialRoom: response.room });
    } catch (error) {
      setRoomError(String(error));
    } finally {
      setIsSubmitting(false);
    }
  }

  function openSocket(
    session: RoomSession,
    options?: { initialRoom?: RoomSnapshot; preserveState?: boolean; reconnectAttempt?: number },
  ) {
    clearReconnectTimer();
    closeSocketSilently();

    const preserveState = options?.preserveState ?? false;
    const reconnectAttempt = options?.reconnectAttempt ?? 0;

    if (!preserveState) {
      setRoomSession(session);
      setRoomSnapshot(options?.initialRoom ?? null);
      setChatMessages([]);
      setSelfReady(false);
      setRoomClosedReason(null);
      roomClosedReasonRef.current = null;
      nextSeqRef.current = 1;
    }

    setRoomError(null);
    setTransportState(reconnectAttempt > 0 ? 'reconnecting' : 'connecting');

    const socket = connectRoomSocket(session);
    socketRef.current = socket;

    socket.onopen = () => {
      if (socketRef.current !== socket) {
        return;
      }
      setTransportState('connected');
      pushEvent(reconnectAttempt > 0 ? `Reconnected to room ${session.roomCode}` : `Connected to room ${session.roomCode}`);
    };

    socket.onmessage = (event) => {
      if (socketRef.current !== socket) {
        return;
      }
      try {
        const message = JSON.parse(event.data as string) as ServerEnvelope;
        void handleServerEnvelope(message);
      } catch (error) {
        pushEvent(`Socket message failed: ${String(error)}`);
      }
    };

    socket.onerror = () => {
      if (socketRef.current === socket) {
        pushEvent('Room socket error');
      }
    };

    socket.onclose = () => {
      if (socketRef.current !== socket) {
        return;
      }
      socketRef.current = null;
      if (manualCloseRef.current) {
        manualCloseRef.current = false;
        return;
      }
      if (roomClosedReasonRef.current) {
        setTransportState('closed');
        return;
      }
      scheduleReconnect(session, reconnectAttempt + 1);
    };
  }

  function scheduleReconnect(session: RoomSession, attempt: number) {
    if (attempt > MAX_RECONNECT_ATTEMPTS) {
      setTransportState('closed');
      setRoomError('Connection lost. The room session could not be restored.');
      pushEvent('Reconnect attempts exhausted');
      return;
    }

    setTransportState('reconnecting');
    pushEvent(`Reconnect attempt ${attempt}/${MAX_RECONNECT_ATTEMPTS}`);
    reconnectTimerRef.current = setTimeout(() => {
      if (roomSessionRef.current?.sessionId !== session.sessionId) {
        return;
      }
      openSocket(session, { preserveState: true, reconnectAttempt: attempt });
    }, RECONNECT_BASE_DELAY_MS * attempt);
  }

  function clearReconnectTimer() {
    if (reconnectTimerRef.current) {
      clearTimeout(reconnectTimerRef.current);
      reconnectTimerRef.current = null;
    }
  }

  function clearHeartbeatTimer() {
    if (heartbeatTimerRef.current) {
      clearInterval(heartbeatTimerRef.current);
      heartbeatTimerRef.current = null;
    }
  }

  function clearCopiedTimer() {
    if (copiedTimerRef.current) {
      clearTimeout(copiedTimerRef.current);
      copiedTimerRef.current = null;
    }
  }

  function closeSocketSilently() {
    clearReconnectTimer();
    if (socketRef.current) {
      manualCloseRef.current = true;
      socketRef.current.close();
      socketRef.current = null;
    }
  }

  function applyRoomSnapshot(snapshot: RoomSnapshot) {
    setRoomSnapshot(snapshot);
    const selfParticipant = roomSessionRef.current
      ? snapshot.participants.find((participant) => participant.sessionId === roomSessionRef.current?.sessionId)
      : undefined;
    setSelfReady(selfParticipant?.ready ?? false);
  }

  async function handleServerEnvelope(message: ServerEnvelope) {
    switch (message.type) {
      case 'welcome': {
        applyRoomSnapshot(message.payload.room);
        setChatMessages((current) => mergeChatMessages(current, message.payload.chatHistory ?? []));
        await syncPlayerFromSnapshot(message.payload.playback);
        pushEvent(`Presence synced for ${message.payload.room.roomCode}`);
        return;
      }
      case 'presence': {
        applyRoomSnapshot(message.payload);
        return;
      }
      case 'chat': {
        setChatMessages((current) => mergeChatMessages(current, message.payload));
        return;
      }
      case 'playback': {
        if (message.payload.streamUrl) {
          setStreamUrl(message.payload.streamUrl);
        }
        lastRoomCommandSeqRef.current = Math.max(lastRoomCommandSeqRef.current, message.payload.seq);
        await applyPlaybackCommand(message.payload);
        pushEvent(`Playback command ${message.payload.action} (${message.payload.seq})`);
        return;
      }
      case 'playback_heartbeat': {
        await applyPlaybackHeartbeat(message.payload);
        return;
      }
      case 'error': {
        setRoomError(message.payload.message);
        pushEvent(`Room error: ${message.payload.message}`);
        return;
      }
      case 'room_closed': {
        clearReconnectTimer();
        setRoomClosedReason(message.payload.reason);
        roomClosedReasonRef.current = message.payload.reason;
        setTransportState('closed');
        pushEvent(`Room closed: ${formatCloseReason(message.payload.reason)}`);
        closeSocketSilently();
        return;
      }
      default:
        return;
    }
  }

  async function syncPlayerFromSnapshot(snapshot: PlayerState) {
    if (!snapshot.activeSource) {
      return;
    }

    setStreamUrl(snapshot.activeSource);
    await tauriPlayer.loadStream(snapshot.activeSource);
    if (snapshot.positionMs > 0) {
      await tauriPlayer.seek(snapshot.positionMs);
    }

    if (snapshot.status === 'playing') {
      await tauriPlayer.play();
    } else if (snapshot.status === 'paused') {
      await tauriPlayer.pause();
    } else if (snapshot.status === 'stopped') {
      await tauriPlayer.stop();
    }
  }

  async function applyPlaybackHeartbeat(heartbeat: PlaybackHeartbeat) {
    const plan = resolveHeartbeatPlan({
      current: playerStateRef.current,
      heartbeat,
      isHost: roomSessionRef.current?.role === 'host',
      lastCommandSeq: lastRoomCommandSeqRef.current,
      lastCorrectionAtMs: lastSyncCorrectionAtRef.current,
      nowMs: Date.now(),
    });

    if (plan.kind === 'ignore') {
      return;
    }

    if (plan.loadSource) {
      setStreamUrl(plan.loadSource);
      await tauriPlayer.loadStream(plan.loadSource);
    }
    if (typeof plan.seekToMs === 'number') {
      await tauriPlayer.seek(plan.seekToMs);
    }
    if (plan.playbackIntent === 'play') {
      await tauriPlayer.play();
    } else if (plan.playbackIntent === 'pause') {
      await tauriPlayer.pause();
    }
    if (typeof plan.playbackRatePercent === 'number') {
      await tauriPlayer.setPlaybackRate(plan.playbackRatePercent);
    }
    if (typeof plan.correctionAtMs === 'number') {
      lastSyncCorrectionAtRef.current = plan.correctionAtMs;
    }
    if (plan.logMessage) {
      pushEvent(plan.logMessage);
    }
  }

  async function applyPlaybackCommand(command: PlaybackCommand) {
    switch (command.action) {
      case 'load_stream':
        if (command.streamUrl) {
          await tauriPlayer.loadStream(command.streamUrl);
          await tauriPlayer.setPlaybackRate(100);
          if (typeof command.positionMs === 'number' && command.positionMs > 0) {
            await tauriPlayer.seek(command.positionMs);
          }
        }
        return;
      case 'play':
        await tauriPlayer.setPlaybackRate(100);
        await tauriPlayer.play();
        return;
      case 'pause':
        await tauriPlayer.setPlaybackRate(100);
        await tauriPlayer.pause();
        return;
      case 'seek':
        await tauriPlayer.setPlaybackRate(100);
        await tauriPlayer.seek(command.positionMs ?? 0);
        return;
      case 'stop':
        await tauriPlayer.setPlaybackRate(100);
        await tauriPlayer.stop();
        return;
      default:
        return;
    }
  }

  async function dispatchPlayback(action: PlaybackAction) {
    try {
      let command: PlaybackCommand;
      switch (action) {
        case 'load_stream':
          if (!streamUrl.trim()) {
            setRoomError('Paste a direct media URL first.');
            return;
          }
          command = buildPlaybackCommand('load_stream', {
            streamUrl: streamUrl.trim(),
            positionMs: 0,
          });
          break;
        case 'seek':
          command = buildPlaybackCommand('seek', { positionMs: seekValue });
          break;
        default:
          command = buildPlaybackCommand(action);
          break;
      }

      await applyPlaybackCommand(command);
      pushEvent(`Local ${action.replace('_', ' ')} issued`);

      if (roomSession && isHost) {
        sendEnvelope(socketRef.current, { type: 'playback_command', payload: command });
      }
    } catch (error) {
      setRoomError(String(error));
    }
  }

  function buildPlaybackCommand(
    action: PlaybackAction,
    overrides?: Partial<Pick<PlaybackCommand, 'positionMs' | 'streamUrl'>>,
  ): PlaybackCommand {
    const command = {
      seq: nextSeqRef.current,
      action,
      positionMs: overrides?.positionMs ?? null,
      streamUrl: overrides?.streamUrl ?? null,
      issuedAtMs: Date.now(),
    } satisfies PlaybackCommand;
    nextSeqRef.current += 1;
    return command;
  }

  function toggleReady() {
    const nextReady = !selfReady;
    setSelfReady(nextReady);
    try {
      sendEnvelope(socketRef.current, {
        type: 'ready_state',
        payload: { ready: nextReady },
      });
      pushEvent(nextReady ? 'Marked ready' : 'Marked not ready');
    } catch (error) {
      setRoomError(String(error));
    }
  }

  function sendChatMessage() {
    if (!chatDraft.trim()) {
      return;
    }
    if (chatTooLong) {
      setRoomError('Chat messages must be 500 characters or less.');
      return;
    }

    try {
      sendEnvelope(socketRef.current, {
        type: 'chat_send',
        payload: { id: createClientMessageId(), text: chatDraft.trim() },
      });
      setChatDraft('');
    } catch (error) {
      setRoomError(String(error));
    }
  }

  function handleCloseRoom() {
    try {
      sendEnvelope(socketRef.current, { type: 'close_room' });
      pushEvent('Host requested room closure');
    } catch (error) {
      setRoomError(String(error));
    }
  }

  async function copyRoomCode() {
    if (!roomSession) {
      return;
    }

    try {
      await navigator.clipboard.writeText(roomSession.roomCode);
      setCopyState('copied');
      pushEvent(`Copied room code ${roomSession.roomCode}`);
    } catch {
      setCopyState('failed');
      pushEvent('Could not copy room code');
    }

    clearCopiedTimer();
    copiedTimerRef.current = setTimeout(() => setCopyState('idle'), 1800);
  }

  function leaveRoom() {
    closeSocketSilently();
    clearHeartbeatTimer();
    setRoomSession(null);
    roomSessionRef.current = null;
    setRoomSnapshot(null);
    setTransportState('idle');
    setRoomClosedReason(null);
    roomClosedReasonRef.current = null;
    setChatMessages([]);
    setSelfReady(false);
    setRoomError(null);
    pushEvent('Room left');
  }

  function pushEvent(message: string) {
    console.info(`[watch-party] ${message}`);
  }

  return (
    <div className={`app-shell ${mode} ${roomSession ? 'in-room' : 'landing'}`}>
      <header className="topbar">
        <div>
          <p className="eyebrow">Bharatiya Watch Party</p>
          <h1>{roomSession ? 'Watch room' : 'Watch together'}</h1>
        </div>
        <div className="topbar-meta">
          {roomSession ? (
            <button type="button" className="ghost-button" onClick={leaveRoom}>
              Leave room
            </button>
          ) : null}
        </div>
      </header>

      {!roomSession ? (
        <main className="landing-layout">
          <section className="landing-main">
            <p className="eyebrow">Direct streams. Shared timeline.</p>
            <h2>Create a room or join with a code.</h2>

            <div className="split-actions">
              <section className="action-pane">
                <div>
                  <p className="eyebrow">Host</p>
                  <h3>Start a room</h3>
                </div>
                <label className="field">
                  <span>Display name</span>
                  <input value={createName} onChange={(event) => setCreateName(event.target.value)} />
                  <small>{createNameError ?? '2-24 letters, numbers, spaces, underscores, or hyphens.'}</small>
                </label>
                <button type="button" onClick={handleCreateRoom} disabled={isSubmitting || Boolean(createNameError)}>
                  {isSubmitting ? 'Starting...' : 'Create room'}
                </button>
              </section>

              <section className="action-pane">
                <div>
                  <p className="eyebrow">Viewer</p>
                  <h3>Join a room</h3>
                </div>
                <label className="field">
                  <span>Room code</span>
                  <input
                    value={joinCode}
                    maxLength={6}
                    onChange={(event) => setJoinCode(formatRoomCodeInput(event.target.value))}
                  />
                  <small>{joinCodeError ?? 'Six-character room code from the host.'}</small>
                </label>
                <label className="field">
                  <span>Display name</span>
                  <input value={joinName} onChange={(event) => setJoinName(event.target.value)} />
                  <small>{joinNameError ?? 'Use a name others can recognize.'}</small>
                </label>
                <button
                  type="button"
                  className="secondary"
                  onClick={handleJoinRoom}
                  disabled={isSubmitting || Boolean(joinCodeError || joinNameError)}
                >
                  {isSubmitting ? 'Joining...' : 'Join room'}
                </button>
              </section>
            </div>

            {playerWarning ? <p className="status-banner warning">{playerWarning}</p> : null}
            {roomError ? (
              <section className="surface-callout error-state compact-state">
                <p className="eyebrow">{roomErrorTitle}</p>
                <h3>{roomError}</h3>
              </section>
            ) : null}

            <section className="player-panel local-player-panel">
              <div className="panel-head">
                <div>
                  <p className="eyebrow">Local player</p>
                  <h2>Test a stream</h2>
                </div>
                <span className={`status-pill ${playerState.status}`}>{statusLabel}</span>
              </div>

              <div className="player-stage cinematic">
                {playerBackend === 'web-video' ? (
                  <video
                    ref={videoRef}
                    className={`video-surface ${playerState.activeSource ? 'active' : ''}`}
                    controls
                    playsInline
                    preload="metadata"
                  />
                ) : null}
                <div>
                  <p className="stage-label">Source</p>
                  <p className="stage-value">{playerState.activeSource ?? 'Load a direct media URL to test the player bridge'}</p>
                </div>
                <div className="stage-grid">
                  <div>
                    <p className="stage-label">Position</p>
                    <p className="stage-value">{playerState.positionMs} ms</p>
                  </div>
                  <div>
                    <p className="stage-label">Tracks</p>
                    <p className="stage-value">
                      {tracks.audio.length} audio / {tracks.subtitles.length} subtitle
                    </p>
                  </div>
                  <div>
                    <p className="stage-label">Speed</p>
                    <p className="stage-value">{playerState.playbackRatePercent}%</p>
                  </div>
                </div>
              </div>

              <label className="field">
                <span>Direct media URL</span>
                <input
                  value={streamUrl}
                  onChange={(event) => setStreamUrl(event.target.value)}
                  placeholder="https://example.com/video.mp4"
                />
              </label>

              <div className="control-row">
                <button type="button" onClick={() => void dispatchPlayback('load_stream')}>
                  Load
                </button>
                <button type="button" onClick={() => void dispatchPlayback('play')}>
                  Play
                </button>
                <button type="button" onClick={() => void dispatchPlayback('pause')}>
                  Pause
                </button>
                <button type="button" onClick={() => void dispatchPlayback('stop')}>
                  Stop
                </button>
              </div>

              <div className="control-grid">
                <label className="field compact">
                  <span>Seek position (ms)</span>
                  <input
                    type="number"
                    value={seekValue}
                    onChange={(event) => setSeekValue(Number(event.target.value))}
                  />
                </label>
                <button type="button" className="secondary" onClick={() => void dispatchPlayback('seek')}>
                  Seek
                </button>
                <label className="field compact">
                  <span>Audio track</span>
                  <select
                    value={playerState.selectedAudioTrack ?? ''}
                    onChange={(event) => {
                      void tauriPlayer.selectAudioTrack(event.target.value);
                    }}
                    disabled={tracks.audio.length === 0}
                  >
                    <option value="">Default</option>
                    {tracks.audio.map((track) => (
                      <option key={track.id} value={track.id}>
                        {track.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field compact">
                  <span>Subtitle track</span>
                  <select
                    value={subtitleValue}
                    onChange={(event) => {
                      void tauriPlayer.selectSubtitleTrack(event.target.value === 'none' ? null : event.target.value);
                    }}
                    disabled={tracks.subtitles.length === 0}
                  >
                    <option value="none">Off</option>
                    {tracks.subtitles.map((track) => (
                      <option key={track.id} value={track.id}>
                        {track.label}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            </section>
          </section>
        </main>
      ) : (
        <main className="workspace">
          <section className="room-summary">
            <div>
              <p className="eyebrow">Room</p>
              <div className="room-code-row">
                <h2>{roomSession.roomCode}</h2>
                <button type="button" className="copy-code-button" onClick={() => void copyRoomCode()}>
                  {copyState === 'copied' ? 'Copied' : copyState === 'failed' ? 'Copy failed' : 'Copy code'}
                </button>
              </div>
            </div>
            <div className="summary-stats">
              <span className="role-chip">{roomSession.role === 'host' ? 'Host controls' : 'Viewer mode'}</span>
              <span>{participantCount} connected</span>
              <span>{viewerCount}/{roomSession.maxViewers} viewers</span>
              <span>{readyCount} ready</span>
            </div>
            <div className="readiness-meter" aria-label={`${readyCount} of ${participantCount} participants ready`}>
              <span style={{ width: `${readyPercent}%` }} />
            </div>
            <div className="summary-actions">
              <button type="button" className="ghost-button" onClick={toggleReady} disabled={!roomInteractive}>
                {selfReady ? 'Set not ready' : 'Set ready'}
              </button>
              {isHost ? (
                <button type="button" className="ghost-danger" onClick={handleCloseRoom} disabled={!roomInteractive}>
                  End room
                </button>
              ) : null}
            </div>
          </section>

          {playerWarning ? <p className="status-banner warning">{playerWarning}</p> : null}
          {roomError ? <p className="status-banner error">{roomError}</p> : null}

          {surfaceState !== 'Playing' && surfaceState !== 'Buffering' && surfaceState !== 'Loading' ? (
            <section className={`surface-callout ${surfaceState.toLowerCase()}-state`}>
              <p className="eyebrow">{surfaceState}</p>
              <h3>{surfaceHeadline(surfaceState, isHost, roomClosedReason)}</h3>
              <p>{surfaceCopy(surfaceState, isHost, readyCount, participantCount, roomClosedReason)}</p>
              {surfaceState === 'Closed' ? (
                <div className="inline-actions">
                  <button type="button" onClick={leaveRoom}>
                    Back to home
                  </button>
                </div>
              ) : null}
            </section>
          ) : null}

          <section className="stage-panel">
            <div className="player-panel">
              <div className="panel-head">
                <div>
                  <p className="eyebrow">Playback</p>
                  <h2>{playerState.activeSource ? 'Watch surface' : 'Lobby stage'}</h2>
                </div>
                <div className="panel-actions">
                  <span className={`status-pill ${playerState.status}`}>{statusLabel}</span>
                  <button
                    type="button"
                    className="mode-button"
                    onClick={() => setMode((current) => (current === 'standard' ? 'theater' : 'standard'))}
                  >
                    {mode === 'standard' ? 'Theater' : 'Standard'}
                  </button>
                </div>
              </div>

              <div className="player-stage cinematic">
                {playerBackend === 'web-video' ? (
                  <video
                    ref={videoRef}
                    className={`video-surface ${playerState.activeSource ? 'active' : ''}`}
                    controls
                    playsInline
                    preload="metadata"
                  />
                ) : null}
                <div>
                  <p className="stage-label">Source</p>
                  <p className="stage-value">{playerState.activeSource ?? 'Waiting for the host to load a direct media URL'}</p>
                </div>
                <div className="stage-grid">
                  <div>
                    <p className="stage-label">Position</p>
                    <p className="stage-value">{playerState.positionMs} ms</p>
                  </div>
                  <div>
                    <p className="stage-label">Tracks</p>
                    <p className="stage-value">
                      {tracks.audio.length} audio / {tracks.subtitles.length} subtitle
                    </p>
                  </div>
                  <div>
                    <p className="stage-label">Sync speed</p>
                    <p className="stage-value">{playerState.playbackRatePercent}%</p>
                  </div>
                </div>
              </div>

              <label className="field">
                <span>Direct media URL</span>
                <input
                  value={streamUrl}
                  onChange={(event) => setStreamUrl(event.target.value)}
                  disabled={!canOperatePlayer}
                  placeholder="https://example.com/stream.m3u8"
                />
              </label>

              <div className="control-row">
                <button type="button" onClick={() => void dispatchPlayback('load_stream')} disabled={!canOperatePlayer}>
                  Load
                </button>
                <button type="button" onClick={() => void dispatchPlayback('play')} disabled={!canOperatePlayer}>
                  Play
                </button>
                <button type="button" onClick={() => void dispatchPlayback('pause')} disabled={!canOperatePlayer}>
                  Pause
                </button>
                <button type="button" onClick={() => void dispatchPlayback('stop')} disabled={!canOperatePlayer}>
                  Stop
                </button>
              </div>

              <div className="control-grid">
                <label className="field compact">
                  <span>Seek position (ms)</span>
                  <input
                    type="number"
                    value={seekValue}
                    onChange={(event) => setSeekValue(Number(event.target.value))}
                    disabled={!canOperatePlayer}
                  />
                </label>
                <button
                  type="button"
                  className="secondary"
                  onClick={() => void dispatchPlayback('seek')}
                  disabled={!canOperatePlayer}
                >
                  Seek
                </button>
                <label className="field compact">
                  <span>Audio track</span>
                  <select
                    value={playerState.selectedAudioTrack ?? ''}
                    onChange={(event) => {
                      void tauriPlayer.selectAudioTrack(event.target.value);
                    }}
                  >
                    {tracks.audio.map((track) => (
                      <option key={track.id} value={track.id}>
                        {track.label}
                      </option>
                    ))}
                  </select>
                </label>
                <label className="field compact">
                  <span>Subtitle track</span>
                  <select
                    value={subtitleValue}
                    onChange={(event) => {
                      void tauriPlayer.selectSubtitleTrack(event.target.value === 'none' ? null : event.target.value);
                    }}
                  >
                    <option value="none">Off</option>
                    {tracks.subtitles.map((track) => (
                      <option key={track.id} value={track.id}>
                        {track.label}
                      </option>
                    ))}
                  </select>
                </label>
              </div>
            </div>

            <aside className="chat-panel room-rail">
              <div className="panel-head slim">
                <div>
                  <p className="eyebrow">Room activity</p>
                  <h2>People and chat</h2>
                </div>
                <span className="chat-count">{chatMessages.length}</span>
              </div>

              <section className="presence-list">
                {(roomSnapshot?.participants ?? []).length === 0 ? (
                  <article className="empty-state">
                    <strong>No participants yet</strong>
                    <p>Share the room code and viewers will appear here.</p>
                  </article>
                ) : (
                  (roomSnapshot?.participants ?? []).map((participant) => (
                    <div
                      key={participant.sessionId}
                      className={`presence-row ${participant.connected ? '' : 'offline'}`}
                    >
                      <div>
                        <strong>
                          {participant.displayName}
                          {participant.sessionId === roomSession.sessionId ? ' (you)' : ''}
                        </strong>
                        <p>{participant.role === 'host' ? 'Host' : participant.connected ? 'Viewer' : 'Viewer offline'}</p>
                      </div>
                      <span className={`presence-chip ${participant.ready ? 'ready' : 'waiting'}`}>
                        {participant.ready ? 'Ready' : participant.connected ? 'Waiting' : 'Offline'}
                      </span>
                    </div>
                  ))
                )}
              </section>

              <section className="chat-messages live-chat">
                {chatMessages.length === 0 ? (
                  <article>
                    <strong>Room chat</strong>
                    <p>
                      {surfaceState === 'Lobby'
                        ? 'Use chat while the room is getting ready.'
                        : 'Messages from the room will appear here once the socket is active.'}
                    </p>
                  </article>
                ) : (
                  chatMessages.map((message) => (
                    <article key={message.id}>
                      <strong>{message.senderDisplayName}</strong>
                      <p>{message.text}</p>
                    </article>
                  ))
                )}
              </section>

              <footer className="chat-input-row active-chat">
                <input
                  value={chatDraft}
                  onChange={(event) => setChatDraft(event.target.value)}
                  placeholder="Send a message to the room"
                  maxLength={520}
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      sendChatMessage();
                    }
                  }}
                  disabled={!canSendChat}
                />
                <div className="chat-send-stack">
                  <span className={chatTooLong ? 'over-limit' : ''}>{chatLength}/500</span>
                  <button type="button" className="secondary" onClick={sendChatMessage} disabled={!canSendChat || chatTooLong}>
                    Send
                  </button>
                </div>
              </footer>
            </aside>
          </section>
        </main>
      )}
    </div>
  );
}

class AppErrorBoundary extends Component<{ children: ReactNode }, { error: Error | null }> {
  state: { error: Error | null } = { error: null };

  static getDerivedStateFromError(error: Error) {
    return { error };
  }

  componentDidCatch(error: Error, info: ErrorInfo) {
    console.error('[watch-party] UI crashed', error, info);
  }

  render() {
    if (this.state.error) {
      return (
        <main className="app-crash">
          <p className="eyebrow">UI fault</p>
          <h1>Something broke in the room surface.</h1>
          <p>{this.state.error.message}</p>
          <button type="button" onClick={() => this.setState({ error: null })}>
            Try again
          </button>
        </main>
      );
    }

    return this.props.children;
  }
}

function buildRoomSession(session: CreateRoomResponse | JoinRoomResponse): RoomSession {
  return {
    roomCode: session.roomCode,
    sessionId: session.sessionId,
    role: session.role,
    maxViewers: session.maxViewers,
  };
}

function createClientMessageId(): string {
  if (globalThis.crypto?.randomUUID) {
    return globalThis.crypto.randomUUID();
  }
  return `${Date.now()}-${Math.random().toString(36).slice(2)}`;
}

function mergeChatMessages(current: ChatMessage[], incoming: ChatMessage | ChatMessage[] | null | undefined): ChatMessage[] {
  const messages = Array.isArray(incoming) ? incoming : incoming ? [incoming] : [];
  if (messages.length === 0) {
    return current;
  }

  const seen = new Set(current.map((message) => message.id));
  const merged = [...current];
  for (const message of messages) {
    if (!message?.id) {
      continue;
    }
    if (seen.has(message.id)) {
      continue;
    }
    seen.add(message.id);
    merged.push(message);
  }
  return merged.slice(-CHAT_HISTORY_LIMIT);
}

function deriveSurfaceState(
  roomSession: RoomSession | null,
  transportState: TransportState,
  roomClosedReason: RoomCloseReason | null,
  playerStatus: PlayerState['status'],
): RoomSurfaceState {
  if (roomClosedReason || transportState === 'closed') {
    return 'Closed';
  }
  if (transportState === 'reconnecting') {
    return 'Reconnecting';
  }
  if (transportState === 'connecting') {
    return 'Joining';
  }
  if (!roomSession) {
    return 'Disconnected';
  }

  switch (playerStatus) {
    case 'loading':
      return 'Loading';
    case 'playing':
      return 'Playing';
    case 'buffering':
      return 'Buffering';
    default:
      return 'Lobby';
  }
}

function formatCloseReason(reason: RoomCloseReason): string {
  switch (reason) {
    case 'closed_by_host':
      return 'closed by host';
    case 'expired':
      return 'expired';
    case 'host_disconnected':
      return 'host disconnected';
    default:
      return String(reason).replace(/_/g, ' ');
  }
}

function deriveRoomErrorTitle(message: string): string {
  const normalized = message.toLowerCase();
  if (normalized.includes('full')) {
    return 'Room full';
  }
  if (normalized.includes('not found')) {
    return 'Invalid room code';
  }
  if (normalized.includes('stream')) {
    return 'Invalid stream';
  }
  return 'Room issue';
}

function getDisplayNameError(value: string): string | null {
  const trimmed = value.trim();
  if (trimmed.length < 2 || trimmed.length > 24) {
    return 'Display name must be between 2 and 24 characters.';
  }
  if (!/^[A-Za-z0-9 _-]+$/.test(trimmed)) {
    return 'Display name can only use letters, numbers, spaces, underscores, or hyphens.';
  }
  return null;
}

function getRoomCodeError(value: string): string | null {
  if (!value.trim()) {
    return 'Enter the room code from the host.';
  }
  if (!/^[A-Z0-9]{6}$/.test(value.trim().toUpperCase())) {
    return 'Room code must be 6 uppercase letters or numbers.';
  }
  return null;
}

function formatRoomCodeInput(value: string): string {
  return value.toUpperCase().replace(/[^A-Z0-9]/g, '').slice(0, 6);
}

function surfaceHeadline(
  surfaceState: RoomSurfaceState,
  isHost: boolean,
  roomClosedReason: RoomCloseReason | null,
): string {
  switch (surfaceState) {
    case 'Lobby':
      return isHost ? 'Room is open and waiting for the host to start playback.' : 'You are in the lobby waiting for the host timeline.';
    case 'Reconnecting':
      return 'Trying to restore the room session.';
    case 'Closed':
      return roomClosedReason ? `This room was ${formatCloseReason(roomClosedReason)}.` : 'This room session is no longer active.';
    default:
      return `${surfaceState} state active.`;
  }
}

function surfaceCopy(
  surfaceState: RoomSurfaceState,
  isHost: boolean,
  readyCount: number,
  participantCount: number,
  roomClosedReason: RoomCloseReason | null,
): string {
  switch (surfaceState) {
    case 'Lobby':
      return isHost
        ? `${readyCount} of ${participantCount} participants are marked ready. Load a direct stream and start when you want.`
        : `${readyCount} of ${participantCount} participants are ready. Chat stays live while you wait for playback.`;
    case 'Reconnecting':
      return 'The desktop client keeps your room context and will retry with the same session id before falling back to a closed state.';
    case 'Closed':
      return roomClosedReason === 'closed_by_host'
        ? 'The host ended the room for every participant.'
        : 'Leave this session to return to the home screen and join or create a new room.';
    default:
      return 'The room surface is transitioning.';
  }
}

