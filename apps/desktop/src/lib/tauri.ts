import { invoke } from '@tauri-apps/api/core';
import { listen, type UnlistenFn } from '@tauri-apps/api/event';

import type { PlayerState, TrackCatalog } from './types';

export const PLAYER_STATE_EVENT = 'player:state';
export const PLAYER_TRACKS_EVENT = 'player:tracks';

export const tauriPlayer = {
  bootstrap: () => invoke<{ state: PlayerState; tracks: TrackCatalog }>('bootstrap_player'),
  loadStream: (url: string) => invoke<PlayerState>('load_stream', { url }),
  play: () => invoke<PlayerState>('play'),
  pause: () => invoke<PlayerState>('pause'),
  seek: (positionMs: number) => invoke<PlayerState>('seek', { positionMs }),
  stop: () => invoke<PlayerState>('stop'),
  state: () => invoke<PlayerState>('player_state'),
  tracks: () => invoke<TrackCatalog>('player_tracks'),
  selectAudioTrack: (trackId: string) => invoke<PlayerState>('select_audio_track', { trackId }),
  selectSubtitleTrack: (trackId: string | null) =>
    invoke<PlayerState>('select_subtitle_track', { trackId }),
};

export function onPlayerState(listener: (state: PlayerState) => void): Promise<UnlistenFn> {
  return listen<PlayerState>(PLAYER_STATE_EVENT, (event) => listener(event.payload));
}

export function onPlayerTracks(listener: (tracks: TrackCatalog) => void): Promise<UnlistenFn> {
  return listen<TrackCatalog>(PLAYER_TRACKS_EVENT, (event) => listener(event.payload));
}