export type PlayerStatus =
  | 'idle'
  | 'loading'
  | 'playing'
  | 'paused'
  | 'buffering'
  | 'stopped'
  | 'error';

export type MediaTrackKind = 'audio' | 'subtitle';

export interface MediaTrack {
  id: string;
  label: string;
  language?: string | null;
  codec?: string | null;
  kind: MediaTrackKind;
  selected: boolean;
}

export interface TrackCatalog {
  audio: MediaTrack[];
  subtitles: MediaTrack[];
}

export interface PlayerState {
  status: PlayerStatus;
  activeSource?: string | null;
  positionMs: number;
  durationMs?: number | null;
  volume: number;
  muted: boolean;
  selectedAudioTrack?: string | null;
  selectedSubtitleTrack?: string | null;
  lastError?: string | null;
}

export type RoomSurfaceState =
  | 'Disconnected'
  | 'Joining'
  | 'Lobby'
  | 'Loading'
  | 'Playing'
  | 'Buffering'
  | 'Reconnecting'
  | 'Closed';