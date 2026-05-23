import type { PlayerState, TrackCatalog } from './types';

export const PLAYER_STATE_EVENT = 'player:state';
export const PLAYER_TRACKS_EVENT = 'player:tracks';

export interface BootstrapPayload {
  state: PlayerState;
  tracks: TrackCatalog;
  backend: string;
  backendWarning?: string | null;
}

type UnlistenFn = () => void;
type PlayerMode = 'native' | 'web';

const emptyTracks: TrackCatalog = { audio: [], subtitles: [] };
const stateSubscribers = new Set<(state: PlayerState) => void>();
const trackSubscribers = new Set<(tracks: TrackCatalog) => void>();

let playerMode: PlayerMode = hasTauriRuntime() ? 'native' : 'web';
let nativeEventsAttached = false;
let webVideoElement: HTMLVideoElement | null = null;
let detachWebVideoListeners: UnlistenFn | null = null;
let webState: PlayerState = {
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

export const tauriPlayer = {
  bootstrap: bootstrapPlayer,
  loadStream: (url: string) => routePlayerCommand('load_stream', { url }, () => webLoadStream(url)),
  play: () => routePlayerCommand('play', undefined, webPlay),
  pause: () => routePlayerCommand('pause', undefined, webPause),
  seek: (positionMs: number) => routePlayerCommand('seek', { positionMs }, () => webSeek(positionMs)),
  stop: () => routePlayerCommand('stop', undefined, webStop),
  state: () => routePlayerCommand('player_state', undefined, async () => webState),
  tracks: () => routePlayerCommand('player_tracks', undefined, async () => emptyTracks),
  setPlaybackRate: (playbackRatePercent: number) =>
    routePlayerCommand(
      'set_playback_rate',
      { playbackRatePercent },
      () => webSetPlaybackRate(playbackRatePercent),
    ),
  selectAudioTrack: (trackId: string) =>
    routePlayerCommand('select_audio_track', { trackId }, async () => webState),
  selectSubtitleTrack: (trackId: string | null) =>
    routePlayerCommand('select_subtitle_track', { trackId }, async () => webState),
};

export function registerWebPlayerElement(element: HTMLVideoElement | null): void {
  detachWebVideoListeners?.();
  detachWebVideoListeners = null;
  webVideoElement = element;

  if (!element) {
    return;
  }

  const sync = () => syncWebStateFromElement(element);
  const fail = () => {
    const message = element.error?.message || 'The browser video player could not load this stream.';
    updateWebState({ status: 'error', lastError: message });
  };
  const listeners: Array<[keyof HTMLMediaElementEventMap, EventListener]> = [
    ['loadedmetadata', sync],
    ['durationchange', sync],
    ['timeupdate', sync],
    ['volumechange', sync],
    ['play', sync],
    ['pause', sync],
    ['playing', sync],
    ['waiting', () => updateWebState({ status: 'buffering' })],
    ['ended', () => updateWebState({ status: 'stopped', positionMs: 0 })],
    ['error', fail],
  ];

  for (const [event, listener] of listeners) {
    element.addEventListener(event, listener);
  }

  if (webState.activeSource && element.src !== webState.activeSource) {
    element.src = webState.activeSource;
  }
  sync();

  detachWebVideoListeners = () => {
    for (const [event, listener] of listeners) {
      element.removeEventListener(event, listener);
    }
  };
}

export function onPlayerState(listener: (state: PlayerState) => void): Promise<UnlistenFn> {
  stateSubscribers.add(listener);
  return Promise.resolve(() => {
    stateSubscribers.delete(listener);
  });
}

export function onPlayerTracks(listener: (tracks: TrackCatalog) => void): Promise<UnlistenFn> {
  trackSubscribers.add(listener);
  return Promise.resolve(() => {
    trackSubscribers.delete(listener);
  });
}

async function bootstrapPlayer(): Promise<BootstrapPayload> {
  if (!hasTauriRuntime()) {
    playerMode = 'web';
    return webBootstrap();
  }

  const payload = await nativeInvoke<BootstrapPayload>('bootstrap_player');
  if (payload.backend === 'libmpv') {
    playerMode = 'native';
    await attachNativeEventBridge();
    return payload;
  }

  playerMode = 'web';
  const fallback = webBootstrap();
  return {
    ...fallback,
    backendWarning: [
      payload.backendWarning,
      'Using the browser video fallback. MP4/WebM and browser-native HLS can play here; DASH still requires native playback.',
    ]
      .filter(Boolean)
      .join(' '),
  };
}

async function routePlayerCommand<T>(
  command: string,
  args: Record<string, unknown> | undefined,
  webFallback: () => Promise<T>,
): Promise<T> {
  if (playerMode === 'native') {
    return nativeInvoke<T>(command, args);
  }

  return webFallback();
}

async function nativeInvoke<T>(command: string, args?: Record<string, unknown>): Promise<T> {
  const { invoke } = await import('@tauri-apps/api/core');
  return invoke<T>(command, args);
}

async function attachNativeEventBridge(): Promise<void> {
  if (nativeEventsAttached) {
    return;
  }

  nativeEventsAttached = true;
  const { listen } = await import('@tauri-apps/api/event');
  await listen<PlayerState>(PLAYER_STATE_EVENT, (event) => emitState(event.payload));
  await listen<TrackCatalog>(PLAYER_TRACKS_EVENT, (event) => emitTracks(event.payload));
}

function webBootstrap(): BootstrapPayload {
  emitState(webState);
  emitTracks(emptyTracks);
  return {
    state: webState,
    tracks: emptyTracks,
    backend: 'web-video',
    backendWarning: hasTauriRuntime()
      ? 'Native libmpv is unavailable, so playback is running through the browser video fallback.'
      : null,
  };
}

async function webLoadStream(url: string): Promise<PlayerState> {
  updateWebState({
    status: 'loading',
    activeSource: url,
    positionMs: 0,
    durationMs: null,
    lastError: null,
  });

  if (!webVideoElement) {
    updateWebState({
      status: 'error',
      lastError: 'The browser video surface is not mounted yet.',
    });
    return webState;
  }

  webVideoElement.src = url;
  webVideoElement.load();
  syncWebStateFromElement(webVideoElement, 'paused');
  return webState;
}

async function webPlay(): Promise<PlayerState> {
  if (!webVideoElement || !webState.activeSource) {
    throw new Error('Load a stream before playing.');
  }

  await webVideoElement.play();
  syncWebStateFromElement(webVideoElement, 'playing');
  return webState;
}

async function webPause(): Promise<PlayerState> {
  if (!webVideoElement || !webState.activeSource) {
    throw new Error('Load a stream before pausing.');
  }

  webVideoElement.pause();
  syncWebStateFromElement(webVideoElement, 'paused');
  return webState;
}

async function webSeek(positionMs: number): Promise<PlayerState> {
  if (!webVideoElement || !webState.activeSource) {
    throw new Error('Load a stream before seeking.');
  }

  webVideoElement.currentTime = Math.max(0, positionMs / 1000);
  syncWebStateFromElement(webVideoElement);
  return webState;
}

async function webStop(): Promise<PlayerState> {
  if (webVideoElement) {
    webVideoElement.pause();
    webVideoElement.removeAttribute('src');
    webVideoElement.load();
  }

  updateWebState({
    status: 'stopped',
    activeSource: null,
    positionMs: 0,
    durationMs: null,
    playbackRatePercent: 100,
    lastError: null,
  });
  return webState;
}

async function webSetPlaybackRate(playbackRatePercent: number): Promise<PlayerState> {
  const normalized = Number.isFinite(playbackRatePercent) ? Math.round(playbackRatePercent) : 100;
  const clamped = Math.max(90, Math.min(110, normalized));
  if (webVideoElement) {
    webVideoElement.playbackRate = clamped / 100;
  }
  updateWebState({ playbackRatePercent: clamped });
  return webState;
}

function syncWebStateFromElement(element: HTMLVideoElement, forcedStatus?: PlayerState['status']): void {
  const durationMs = Number.isFinite(element.duration) ? Math.round(element.duration * 1000) : null;
  const status =
    forcedStatus ??
    (element.networkState === HTMLMediaElement.NETWORK_LOADING && element.readyState < HTMLMediaElement.HAVE_FUTURE_DATA
      ? 'buffering'
      : element.paused
        ? 'paused'
        : 'playing');

  updateWebState({
    status,
    activeSource: webState.activeSource ?? element.currentSrc ?? element.src ?? null,
    positionMs: Math.round(element.currentTime * 1000),
    durationMs,
    volume: Math.round(element.volume * 100),
    muted: element.muted,
    playbackRatePercent: Math.round(element.playbackRate * 100),
    lastError: null,
  });
}

function updateWebState(next: Partial<PlayerState>): void {
  webState = { ...webState, ...next };
  emitState(webState);
}

function emitState(state: PlayerState): void {
  for (const subscriber of stateSubscribers) {
    subscriber(state);
  }
}

function emitTracks(tracks: TrackCatalog): void {
  for (const subscriber of trackSubscribers) {
    subscriber(tracks);
  }
}

function hasTauriRuntime(): boolean {
  if (typeof window === 'undefined') {
    return false;
  }

  const candidate = window as Window & {
    __TAURI_INTERNALS__?: unknown;
    __TAURI__?: unknown;
  };
  return Boolean(candidate.__TAURI_INTERNALS__ || candidate.__TAURI__);
}
