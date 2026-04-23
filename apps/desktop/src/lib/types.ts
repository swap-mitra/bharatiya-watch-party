export type PlayerStatus =
  | 'idle'
  | 'loading'
  | 'playing'
  | 'paused'
  | 'buffering'
  | 'stopped'
  | 'error';

export type MediaTrackKind = 'audio' | 'subtitle';
export type ParticipantRole = 'host' | 'viewer';
export type PlaybackAction = 'load_stream' | 'play' | 'pause' | 'seek' | 'stop';
export type RoomCloseReason = 'host_disconnected' | 'expired' | 'closed_by_host';
export type TransportState = 'idle' | 'connecting' | 'connected' | 'closed';

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

export interface Participant {
  sessionId: string;
  displayName: string;
  role: ParticipantRole;
  connected: boolean;
  ready: boolean;
}

export interface RoomSnapshot {
  roomCode: string;
  hostSessionId: string;
  maxViewers: number;
  participants: Participant[];
}

export interface RoomSession {
  roomCode: string;
  sessionId: string;
  role: ParticipantRole;
  maxViewers: number;
}

export interface CreateRoomResponse extends RoomSession {
  expiresInSeconds: number;
}

export interface JoinRoomResponse extends RoomSession {
  room: RoomSnapshot;
}

export interface PlaybackCommand {
  seq: number;
  action: PlaybackAction;
  positionMs?: number | null;
  streamUrl?: string | null;
  issuedAtMs: number;
}

export interface ChatMessage {
  id: string;
  senderSessionId: string;
  senderDisplayName: string;
  text: string;
  sentAtMs: number;
}

export type ClientEnvelope =
  | { type: 'ping' }
  | { type: 'ready_state'; payload: { ready: boolean } }
  | { type: 'chat_send'; payload: { text: string } }
  | { type: 'playback_command'; payload: PlaybackCommand };

export type ServerEnvelope =
  | {
      type: 'welcome';
      payload: {
        room: RoomSnapshot;
        playback: PlayerState;
        selfSessionId: string;
      };
    }
  | { type: 'presence'; payload: RoomSnapshot }
  | { type: 'chat'; payload: ChatMessage }
  | { type: 'playback'; payload: PlaybackCommand }
  | { type: 'error'; payload: { code: string; message: string } }
  | { type: 'room_closed'; payload: { reason: RoomCloseReason } };

export type RoomSurfaceState =
  | 'Disconnected'
  | 'Joining'
  | 'Lobby'
  | 'Loading'
  | 'Playing'
  | 'Buffering'
  | 'Reconnecting'
  | 'Closed';
