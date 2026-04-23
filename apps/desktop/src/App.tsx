import { useEffect, useMemo, useRef, useState } from 'react';

import { connectRoomSocket, createRoom, joinRoom, sendEnvelope } from './lib/roomClient';
import { onPlayerState, onPlayerTracks, tauriPlayer } from './lib/tauri';
import type {
  ChatMessage,
  CreateRoomResponse,
  JoinRoomResponse,
  PlaybackAction,
  PlaybackCommand,
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
  selectedAudioTrack: null,
  selectedSubtitleTrack: null,
  lastError: null,
};

const initialTracks: TrackCatalog = { audio: [], subtitles: [] };

export default function App() {
  const socketRef = useRef<WebSocket | null>(null);
  const manualCloseRef = useRef(false);
  const nextSeqRef = useRef(1);

  const [playerState, setPlayerState] = useState<PlayerState>(initialPlayerState);
  const [tracks, setTracks] = useState<TrackCatalog>(initialTracks);
  const [mode, setMode] = useState<'standard' | 'theater'>('standard');
  const [roomSession, setRoomSession] = useState<RoomSession | null>(null);
  const [roomSnapshot, setRoomSnapshot] = useState<RoomSnapshot | null>(null);
  const [transportState, setTransportState] = useState<TransportState>('idle');
  const [roomClosedReason, setRoomClosedReason] = useState<RoomCloseReason | null>(null);
  const [eventLog, setEventLog] = useState<string[]>(['Desktop shell ready']);
  const [chatMessages, setChatMessages] = useState<ChatMessage[]>([]);
  const [streamUrl, setStreamUrl] = useState(
    'https://demo.unified-streaming.com/k8s/features/stable/video/tears-of-steel/tears-of-steel.ism/.mpd',
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
      manualCloseRef.current = true;
      socketRef.current?.close();
      socketRef.current = null;
      void unlisten.then((handlers) => handlers.forEach((handler) => handler()));
    };
  }, []);

  const isHost = roomSession?.role === 'host';
  const participantCount = roomSnapshot?.participants.length ?? 0;
  const readyCount = roomSnapshot?.participants.filter((participant) => participant.ready).length ?? 0;
  const viewerCount =
    roomSnapshot?.participants.filter((participant) => participant.role === 'viewer').length ?? 0;
  const canOperatePlayer = !roomSession || isHost;
  const subtitleValue = playerState.selectedSubtitleTrack ?? 'none';
  const statusLabel = useMemo(
    () => playerState.status.replace(/(^\w)/, (letter) => letter.toUpperCase()),
    [playerState.status],
  );
  const surfaceState = deriveSurfaceState(roomSession, transportState, roomClosedReason, playerState.status);

  async function handleCreateRoom() {
    if (!createName.trim()) {
      setRoomError('Enter a display name for the host.');
      return;
    }

    setIsSubmitting(true);
    setRoomError(null);
    try {
      const response = await createRoom(createName.trim());
      setChatMessages([]);
      pushEvent(`Room ${response.roomCode} created`);
      connectSocket(response);
    } catch (error) {
      setRoomError(String(error));
    } finally {
      setIsSubmitting(false);
    }
  }

  async function handleJoinRoom() {
    if (!joinName.trim() || !joinCode.trim()) {
      setRoomError('Enter a room code and display name to join.');
      return;
    }

    setIsSubmitting(true);
    setRoomError(null);
    try {
      const response = await joinRoom(joinCode.trim().toUpperCase(), joinName.trim());
      setChatMessages([]);
      pushEvent(`Joining room ${response.roomCode}`);
      connectSocket(response, response.room);
    } catch (error) {
      setRoomError(String(error));
    } finally {
      setIsSubmitting(false);
    }
  }

  function connectSocket(session: CreateRoomResponse | JoinRoomResponse, initialRoom?: RoomSnapshot) {
    manualCloseRef.current = true;
    socketRef.current?.close();
    manualCloseRef.current = false;

    const nextSession: RoomSession = {
      roomCode: session.roomCode,
      sessionId: session.sessionId,
      role: session.role,
      maxViewers: session.maxViewers,
    };

    nextSeqRef.current = 1;
    setRoomSession(nextSession);
    setRoomSnapshot(initialRoom ?? null);
    setSelfReady(false);
    setRoomClosedReason(null);
    setTransportState('connecting');
    setRoomError(null);

    const socket = connectRoomSocket(nextSession);
    socketRef.current = socket;

    socket.onopen = () => {
      setTransportState('connected');
      pushEvent(`Connected to room ${nextSession.roomCode}`);
    };

    socket.onmessage = (event) => {
      try {
        const message = JSON.parse(event.data as string) as ServerEnvelope;
        void handleServerEnvelope(message);
      } catch (error) {
        pushEvent(`Socket message failed: ${String(error)}`);
      }
    };

    socket.onerror = () => {
      pushEvent('Room socket error');
    };

    socket.onclose = () => {
      socketRef.current = null;
      if (manualCloseRef.current) {
        manualCloseRef.current = false;
        return;
      }
      setTransportState('closed');
      pushEvent('Room connection closed');
    };
  }

  async function handleServerEnvelope(message: ServerEnvelope) {
    switch (message.type) {
      case 'welcome': {
        setRoomSnapshot(message.payload.room);
        await syncPlayerFromSnapshot(message.payload.playback);
        pushEvent(`Presence synced for ${message.payload.room.roomCode}`);
        return;
      }
      case 'presence': {
        setRoomSnapshot(message.payload);
        return;
      }
      case 'chat': {
        setChatMessages((current) => [...current.slice(-39), message.payload]);
        return;
      }
      case 'playback': {
        if (message.payload.streamUrl) {
          setStreamUrl(message.payload.streamUrl);
        }
        await applyPlaybackCommand(message.payload);
        pushEvent(`Playback command ${message.payload.action} (${message.payload.seq})`);
        return;
      }
      case 'error': {
        setRoomError(message.payload.message);
        pushEvent(`Room error: ${message.payload.message}`);
        return;
      }
      case 'room_closed': {
        setRoomClosedReason(message.payload.reason);
        setTransportState('closed');
        pushEvent(`Room closed: ${message.payload.reason.replace(/_/g, ' ')}`);
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

  async function applyPlaybackCommand(command: PlaybackCommand) {
    switch (command.action) {
      case 'load_stream':
        if (command.streamUrl) {
          await tauriPlayer.loadStream(command.streamUrl);
          if (typeof command.positionMs === 'number' && command.positionMs > 0) {
            await tauriPlayer.seek(command.positionMs);
          }
        }
        return;
      case 'play':
        await tauriPlayer.play();
        return;
      case 'pause':
        await tauriPlayer.pause();
        return;
      case 'seek':
        await tauriPlayer.seek(command.positionMs ?? 0);
        return;
      case 'stop':
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

    try {
      sendEnvelope(socketRef.current, {
        type: 'chat_send',
        payload: { text: chatDraft.trim() },
      });
      setChatDraft('');
    } catch (error) {
      setRoomError(String(error));
    }
  }

  function leaveRoom() {
    manualCloseRef.current = true;
    socketRef.current?.close();
    socketRef.current = null;
    setRoomSession(null);
    setRoomSnapshot(null);
    setTransportState('idle');
    setRoomClosedReason(null);
    setChatMessages([]);
    setSelfReady(false);
    setRoomError(null);
    pushEvent('Room left');
  }

  function pushEvent(message: string) {
    setEventLog((current) => [message, ...current].slice(0, 10));
  }

  return (
    <div className={`app-shell ${mode} ${roomSession ? 'in-room' : 'landing'}`}>
      <header className="topbar">
        <div>
          <p className="eyebrow">Bharatiya Watch Party</p>
          <h1>{roomSession ? 'Room session' : 'Desktop watch party client'}</h1>
        </div>
        <div className="topbar-meta">
          <span>Surface {surfaceState}</span>
          <span>{transportState === 'connected' ? 'Realtime linked' : 'Local mode'}</span>
          <span>{playerBackend === 'libmpv' ? 'Native libmpv' : 'Mock player'}</span>
          <button
            type="button"
            className="ghost-button"
            onClick={() => setMode((current) => (current === 'standard' ? 'theater' : 'standard'))}
          >
            {mode === 'standard' ? 'Theater mode' : 'Standard mode'}
          </button>
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
            <p className="eyebrow">Private rooms, direct streams, tight sync</p>
            <h2>Create a room or step into one with a short code.</h2>
            <p className="lead-copy">
              The desktop shell is now wired for room lifecycle, host-controlled playback, readiness, and text chat.
            </p>

            <div className="split-actions">
              <section className="action-pane">
                <div>
                  <p className="eyebrow">Host</p>
                  <h3>Start a room</h3>
                </div>
                <label className="field">
                  <span>Display name</span>
                  <input value={createName} onChange={(event) => setCreateName(event.target.value)} />
                </label>
                <button type="button" onClick={handleCreateRoom} disabled={isSubmitting}>
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
                  <input value={joinCode} onChange={(event) => setJoinCode(event.target.value.toUpperCase())} />
                </label>
                <label className="field">
                  <span>Display name</span>
                  <input value={joinName} onChange={(event) => setJoinName(event.target.value)} />
                </label>
                <button type="button" className="secondary" onClick={handleJoinRoom} disabled={isSubmitting}>
                  {isSubmitting ? 'Joining...' : 'Join room'}
                </button>
              </section>
            </div>

            {playerWarning ? <p className="status-banner warning">{playerWarning}</p> : null}
            {roomError ? <p className="status-banner error">{roomError}</p> : null}
          </section>

          <aside className="landing-side">
            <div className="landing-rail">
              <p className="eyebrow">Current foundation</p>
              <ul className="feature-list">
                <li>Rust room service with host authority and viewer limits</li>
                <li>Tauri player bridge ready for real `libmpv` integration</li>
                <li>Live room shell with chat, presence, and theater layout</li>
              </ul>
            </div>
            <div className="landing-rail subtle">
              <p className="eyebrow">Playback backend</p>
              <ul className="feature-list compact-list">
                <li>{playerBackend === 'libmpv' ? 'Native playback is available through libmpv' : 'Mock harness active until libmpv is installed'}</li>
                <li>Native playback currently opens through mpv’s own window while the desktop shell controls it</li>
                <li>Room and player contracts stay the same regardless of backend mode</li>
              </ul>
            </div>
          </aside>
        </main>
      ) : (
        <main className="workspace">
          <section className="room-summary">
            <div>
              <p className="eyebrow">Room</p>
              <h2>{roomSession.roomCode}</h2>
            </div>
            <div className="summary-stats">
              <span>{roomSession.role === 'host' ? 'Host' : 'Viewer'}</span>
              <span>{participantCount} connected</span>
              <span>{viewerCount}/{roomSession.maxViewers} viewers</span>
              <span>{readyCount} ready</span>
            </div>
            <div className="summary-actions">
              <button
                type="button"
                className="ghost-button"
                onClick={toggleReady}
                disabled={transportState !== 'connected' || roomClosedReason !== null}
              >
                {selfReady ? 'Set not ready' : 'Set ready'}
              </button>
              {roomClosedReason ? (
                <span className="status-banner warning">Room closed: {roomClosedReason.replace(/_/g, ' ')}</span>
              ) : null}
            </div>
          </section>

          {playerWarning ? <p className="status-banner warning">{playerWarning}</p> : null}
          {roomError ? <p className="status-banner error">{roomError}</p> : null}

          <section className="stage-panel">
            <div className="player-panel">
              <div className="panel-head">
                <div>
                  <p className="eyebrow">Playback</p>
                  <h2>{playerState.activeSource ? 'Watch surface' : 'Lobby stage'}</h2>
                </div>
                <span className={`status-pill ${playerState.status}`}>{statusLabel}</span>
              </div>

              <div className="player-stage cinematic">
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
                    <p className="stage-value">{tracks.audio.length} audio / {tracks.subtitles.length} subtitle</p>
                  </div>
                </div>
                <p className="stage-note">
                  {isHost
                    ? 'Host commands replicate through the room socket. Viewers mirror playback and keep chat live beside the stage.'
                    : 'Viewer mode follows host-issued playback commands. Local controls stay read-only inside the room.'}
                </p>
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
              </div>

              <section className="presence-list">
                {(roomSnapshot?.participants ?? []).map((participant) => (
                  <div key={participant.sessionId} className="presence-row">
                    <div>
                      <strong>{participant.displayName}</strong>
                      <p>{participant.role === 'host' ? 'Host' : 'Viewer'}</p>
                    </div>
                    <span className={`presence-chip ${participant.ready ? 'ready' : 'waiting'}`}>
                      {participant.ready ? 'Ready' : 'Waiting'}
                    </span>
                  </div>
                ))}
              </section>

              <section className="chat-messages live-chat">
                {chatMessages.length === 0 ? (
                  <article>
                    <strong>Room chat</strong>
                    <p>Messages from the room will appear here once the socket is active.</p>
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
                  onKeyDown={(event) => {
                    if (event.key === 'Enter') {
                      event.preventDefault();
                      sendChatMessage();
                    }
                  }}
                  disabled={transportState !== 'connected' || roomClosedReason !== null}
                />
                <button
                  type="button"
                  className="secondary"
                  onClick={sendChatMessage}
                  disabled={transportState !== 'connected' || roomClosedReason !== null}
                >
                  Send
                </button>
              </footer>
            </aside>
          </section>

          <footer className="bottom-strip">
            <section>
              <p className="eyebrow">Event log</p>
              <ul>
                {eventLog.map((entry) => (
                  <li key={entry}>{entry}</li>
                ))}
              </ul>
            </section>
            <section>
              <p className="eyebrow">Session detail</p>
              <p>{transportState === 'connected' ? 'Connected to signaling service' : 'Socket not attached'}</p>
              <p>{roomSession.role === 'host' ? 'Host authority enabled' : 'Viewer following host timeline'}</p>
              <p>{playerBackend === 'libmpv' ? 'Playback opens through native libmpv' : 'Playback is running on the mock harness'}</p>
            </section>
          </footer>
        </main>
      )}
    </div>
  );
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
    case 'error':
      return 'Reconnecting';
    default:
      return 'Lobby';
  }
}
