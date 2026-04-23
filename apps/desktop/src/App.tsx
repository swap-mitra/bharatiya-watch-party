import { useEffect, useMemo, useState } from 'react';

import { onPlayerState, onPlayerTracks, tauriPlayer } from './lib/tauri';
import type { PlayerState, RoomSurfaceState, TrackCatalog } from './lib/types';
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
  const [streamUrl, setStreamUrl] = useState(
    'https://demo.unified-streaming.com/k8s/features/stable/video/tears-of-steel/tears-of-steel.ism/.mpd',
  );
  const [playerState, setPlayerState] = useState<PlayerState>(initialPlayerState);
  const [tracks, setTracks] = useState<TrackCatalog>(initialTracks);
  const [mode, setMode] = useState<'standard' | 'theater'>('standard');
  const [surfaceState, setSurfaceState] = useState<RoomSurfaceState>('Disconnected');
  const [eventLog, setEventLog] = useState<string[]>(['Desktop harness ready']);
  const [seekValue, setSeekValue] = useState(120000);

  useEffect(() => {
    tauriPlayer
      .bootstrap()
      .then(({ state, tracks: initialCatalog }) => {
        setPlayerState(state);
        setTracks(initialCatalog);
      })
      .catch((error: unknown) => {
        setEventLog((current) => [`Bootstrap failed: ${String(error)}`, ...current]);
      });

    const unlisten = Promise.all([
      onPlayerState((state) => {
        setPlayerState(state);
        setSurfaceState(mapPlayerToSurfaceState(state.status));
        setEventLog((current) => [`State -> ${state.status} @ ${state.positionMs}ms`, ...current].slice(0, 8));
      }),
      onPlayerTracks((nextTracks) => {
        setTracks(nextTracks);
        setEventLog((current) => ['Track catalog updated', ...current].slice(0, 8));
      }),
    ]);

    return () => {
      void unlisten.then((handlers) => handlers.forEach((handler) => handler()));
    };
  }, []);

  const subtitleValue = playerState.selectedSubtitleTrack ?? 'none';
  const statusLabel = useMemo(
    () => playerState.status.replace(/(^\w)/, (letter) => letter.toUpperCase()),
    [playerState.status],
  );

  async function runAction(label: string, action: () => Promise<unknown>) {
    try {
      await action();
      setEventLog((current) => [label, ...current].slice(0, 8));
    } catch (error) {
      setEventLog((current) => [`${label} failed: ${String(error)}`, ...current].slice(0, 8));
    }
  }

  return (
    <div className={`app-shell ${mode}`}>
      <header className="topbar">
        <div>
          <p className="eyebrow">Bharatiya Watch Party</p>
          <h1>Desktop playback harness</h1>
        </div>
        <div className="topbar-meta">
          <span>Surface {surfaceState}</span>
          <span>Status {statusLabel}</span>
          <button
            type="button"
            className="ghost-button"
            onClick={() => setMode((current) => (current === 'standard' ? 'theater' : 'standard'))}
          >
            {mode === 'standard' ? 'Theater mode' : 'Standard mode'}
          </button>
        </div>
      </header>

      <main className="workspace">
        <section className="stage-panel">
          <div className="player-panel">
            <div className="panel-head">
              <div>
                <p className="eyebrow">Native player bridge</p>
                <h2>Rust-driven playback contract</h2>
              </div>
              <span className={`status-pill ${playerState.status}`}>{statusLabel}</span>
            </div>

            <div className="player-stage">
              <div>
                <p className="stage-label">Source</p>
                <p className="stage-value">{playerState.activeSource ?? 'No stream loaded'}</p>
              </div>
              <div className="stage-grid">
                <div>
                  <p className="stage-label">Position</p>
                  <p className="stage-value">{playerState.positionMs} ms</p>
                </div>
                <div>
                  <p className="stage-label">Volume</p>
                  <p className="stage-value">{playerState.volume}%</p>
                </div>
              </div>
              <p className="stage-note">
                The harness is wired to the Rust player adapter surface. `libmpv` integration slots in behind the same commands and events.
              </p>
            </div>

            <label className="field">
              <span>Direct media URL</span>
              <input
                value={streamUrl}
                onChange={(event) => setStreamUrl(event.target.value)}
                placeholder="https://example.com/stream.m3u8"
              />
            </label>

            <div className="control-row">
              <button type="button" onClick={() => runAction('Load stream requested', () => tauriPlayer.loadStream(streamUrl))}>
                Load
              </button>
              <button type="button" onClick={() => runAction('Play requested', () => tauriPlayer.play())}>
                Play
              </button>
              <button type="button" onClick={() => runAction('Pause requested', () => tauriPlayer.pause())}>
                Pause
              </button>
              <button type="button" onClick={() => runAction('Stop requested', () => tauriPlayer.stop())}>
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
              <button
                type="button"
                className="secondary"
                onClick={() => runAction('Seek requested', () => tauriPlayer.seek(seekValue))}
              >
                Seek
              </button>
              <label className="field compact">
                <span>Audio track</span>
                <select
                  value={playerState.selectedAudioTrack ?? ''}
                  onChange={(event) => {
                    void runAction('Audio track changed', () => tauriPlayer.selectAudioTrack(event.target.value));
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
                    void runAction('Subtitle track changed', () =>
                      tauriPlayer.selectSubtitleTrack(event.target.value === 'none' ? null : event.target.value),
                    );
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

          <aside className="chat-panel">
            <div className="panel-head slim">
              <div>
                <p className="eyebrow">Realtime chat</p>
                <h2>Layout target</h2>
              </div>
            </div>
            <div className="chat-messages">
              <article>
                <strong>Host</strong>
                <p>Paste a direct stream URL, verify tracks, then wire this shell to the room sync service.</p>
              </article>
              <article>
                <strong>Viewer</strong>
                <p>Theater mode moves this panel below the player. Standard mode keeps it beside the stage.</p>
              </article>
            </div>
            <footer className="chat-input-row">
              <input disabled value="Sprint 4 will connect this to room chat" readOnly />
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
            <p className="eyebrow">Track summary</p>
            <p>
              {tracks.audio.length} audio track(s), {tracks.subtitles.length} subtitle track(s)
            </p>
          </section>
        </footer>
      </main>
    </div>
  );
}

function mapPlayerToSurfaceState(status: PlayerState['status']): RoomSurfaceState {
  switch (status) {
    case 'loading':
      return 'Loading';
    case 'playing':
      return 'Playing';
    case 'buffering':
      return 'Buffering';
    case 'paused':
    case 'idle':
    case 'stopped':
      return 'Lobby';
    case 'error':
      return 'Reconnecting';
    default:
      return 'Disconnected';
  }
}
