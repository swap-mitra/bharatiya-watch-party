use std::sync::Arc;

use app_core::{
    AppError, AppResult, LoadStreamRequest, MediaTrack, MediaTrackKind, PlayerAdapter, PlayerState,
    PlayerStatus, TrackCatalog, validation,
};
use async_trait::async_trait;
use parking_lot::RwLock;
use tauri::{AppHandle, Emitter, State};

const PLAYER_STATE_EVENT: &str = "player:state";
const PLAYER_TRACKS_EVENT: &str = "player:tracks";

struct DesktopState {
    player: Arc<MpvAdapter>,
}

#[derive(Default)]
struct MpvAdapter {
    state: RwLock<PlayerState>,
    tracks: RwLock<TrackCatalog>,
}

impl MpvAdapter {
    fn derive_tracks(url: &str) -> TrackCatalog {
        let lower = url.to_ascii_lowercase();
        let multi_track =
            lower.ends_with(".m3u8") || lower.ends_with(".mpd") || lower.contains("tears-of-steel");
        let audio = vec![
            MediaTrack {
                id: "audio-main".into(),
                label: "Main audio".into(),
                language: Some("und".into()),
                codec: Some("aac".into()),
                kind: MediaTrackKind::Audio,
                selected: true,
            },
            MediaTrack {
                id: "audio-alt".into(),
                label: "Alternate audio".into(),
                language: Some("hi".into()),
                codec: Some("aac".into()),
                kind: MediaTrackKind::Audio,
                selected: false,
            },
        ];
        let subtitles = if multi_track {
            vec![
                MediaTrack {
                    id: "sub-en".into(),
                    label: "English captions".into(),
                    language: Some("en".into()),
                    codec: Some("webvtt".into()),
                    kind: MediaTrackKind::Subtitle,
                    selected: true,
                },
                MediaTrack {
                    id: "sub-hi".into(),
                    label: "Hindi captions".into(),
                    language: Some("hi".into()),
                    codec: Some("webvtt".into()),
                    kind: MediaTrackKind::Subtitle,
                    selected: false,
                },
            ]
        } else {
            Vec::new()
        };

        TrackCatalog { audio, subtitles }
    }
}

#[async_trait]
impl PlayerAdapter for MpvAdapter {
    async fn load_stream(&self, request: LoadStreamRequest) -> AppResult<PlayerState> {
        request.validate()?;
        let catalog = Self::derive_tracks(&request.url);
        let selected_audio_track = catalog
            .audio
            .iter()
            .find(|track| track.selected)
            .map(|track| track.id.clone());
        let selected_subtitle_track = catalog
            .subtitles
            .iter()
            .find(|track| track.selected)
            .map(|track| track.id.clone());
        *self.tracks.write() = catalog;
        let mut state = self.state.write();
        *state = PlayerState {
            status: PlayerStatus::Paused,
            active_source: Some(request.url),
            position_ms: 0,
            duration_ms: None,
            volume: 100,
            muted: false,
            selected_audio_track,
            selected_subtitle_track,
            last_error: None,
        };
        Ok(state.clone())
    }

    async fn play(&self) -> AppResult<PlayerState> {
        let mut state = self.state.write();
        if state.active_source.is_none() {
            return Err(AppError::Validation("load a stream before playing".into()));
        }
        state.status = PlayerStatus::Playing;
        Ok(state.clone())
    }

    async fn pause(&self) -> AppResult<PlayerState> {
        let mut state = self.state.write();
        if state.active_source.is_none() {
            return Err(AppError::Validation("load a stream before pausing".into()));
        }
        state.status = PlayerStatus::Paused;
        Ok(state.clone())
    }

    async fn seek(&self, position_ms: u64) -> AppResult<PlayerState> {
        let mut state = self.state.write();
        if state.active_source.is_none() {
            return Err(AppError::Validation("load a stream before seeking".into()));
        }
        state.position_ms = position_ms;
        Ok(state.clone())
    }

    async fn stop(&self) -> AppResult<PlayerState> {
        let mut state = self.state.write();
        state.status = PlayerStatus::Stopped;
        state.position_ms = 0;
        Ok(state.clone())
    }

    async fn state(&self) -> AppResult<PlayerState> {
        Ok(self.state.read().clone())
    }

    async fn tracks(&self) -> AppResult<TrackCatalog> {
        Ok(self.tracks.read().clone())
    }

    async fn select_audio_track(&self, track_id: String) -> AppResult<PlayerState> {
        {
            let mut tracks = self.tracks.write();
            let mut found = false;
            for track in &mut tracks.audio {
                track.selected = track.id == track_id;
                found |= track.selected;
            }
            if !found {
                return Err(AppError::Validation("unknown audio track".into()));
            }
        }
        let mut state = self.state.write();
        state.selected_audio_track = Some(track_id);
        Ok(state.clone())
    }

    async fn select_subtitle_track(&self, track_id: Option<String>) -> AppResult<PlayerState> {
        {
            let mut tracks = self.tracks.write();
            if let Some(track_id) = &track_id {
                let mut found = false;
                for track in &mut tracks.subtitles {
                    track.selected = &track.id == track_id;
                    found |= track.selected;
                }
                if !found {
                    return Err(AppError::Validation("unknown subtitle track".into()));
                }
            } else {
                for track in &mut tracks.subtitles {
                    track.selected = false;
                }
            }
        }
        let mut state = self.state.write();
        state.selected_subtitle_track = track_id;
        Ok(state.clone())
    }
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    state: PlayerState,
    tracks: TrackCatalog,
}

#[tauri::command]
async fn bootstrap_player(state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    Ok(BootstrapPayload {
        state: state.player.state().await.map_err(to_string_error)?,
        tracks: state.player.tracks().await.map_err(to_string_error)?,
    })
}

#[tauri::command]
async fn load_stream(
    app: AppHandle,
    state: State<'_, DesktopState>,
    url: String,
) -> Result<PlayerState, String> {
    validation::validate_stream_url(&url).map_err(to_string_error)?;
    let next_state = state
        .player
        .load_stream(LoadStreamRequest { url })
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &state.player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn play(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let next_state = state.player.play().await.map_err(to_string_error)?;
    emit_state(&app, &next_state).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn pause(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let next_state = state.player.pause().await.map_err(to_string_error)?;
    emit_state(&app, &next_state).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn seek(
    app: AppHandle,
    state: State<'_, DesktopState>,
    position_ms: u64,
) -> Result<PlayerState, String> {
    let next_state = state
        .player
        .seek(position_ms)
        .await
        .map_err(to_string_error)?;
    emit_state(&app, &next_state).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn stop(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let next_state = state.player.stop().await.map_err(to_string_error)?;
    emit_state(&app, &next_state).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn player_state(state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    state.player.state().await.map_err(to_string_error)
}

#[tauri::command]
async fn player_tracks(state: State<'_, DesktopState>) -> Result<TrackCatalog, String> {
    state.player.tracks().await.map_err(to_string_error)
}

#[tauri::command]
async fn select_audio_track(
    app: AppHandle,
    state: State<'_, DesktopState>,
    track_id: String,
) -> Result<PlayerState, String> {
    let next_state = state
        .player
        .select_audio_track(track_id)
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &state.player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn select_subtitle_track(
    app: AppHandle,
    state: State<'_, DesktopState>,
    track_id: Option<String>,
) -> Result<PlayerState, String> {
    let next_state = state
        .player
        .select_subtitle_track(track_id)
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &state.player).map_err(to_string_error)?;
    Ok(next_state)
}

fn emit_snapshot(app: &AppHandle, player: &Arc<MpvAdapter>) -> AppResult<()> {
    let state = player.state.read().clone();
    let tracks = player.tracks.read().clone();
    emit_state(app, &state)?;
    app.emit(PLAYER_TRACKS_EVENT, tracks)
        .map_err(|err| AppError::Transport(err.to_string()))
}

fn emit_state(app: &AppHandle, state: &PlayerState) -> AppResult<()> {
    app.emit(PLAYER_STATE_EVENT, state)
        .map_err(|err| AppError::Transport(err.to_string()))
}

fn to_string_error(error: AppError) -> String {
    error.to_string()
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .manage(DesktopState {
            player: Arc::new(MpvAdapter::default()),
        })
        .invoke_handler(tauri::generate_handler![
            bootstrap_player,
            load_stream,
            play,
            pause,
            seek,
            stop,
            player_state,
            player_tracks,
            select_audio_track,
            select_subtitle_track,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

fn main() {
    run();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn adapter_loads_stream_and_selects_tracks() {
        let player = MpvAdapter::default();
        let state = player
            .load_stream(LoadStreamRequest {
                url: "https://example.com/video.m3u8".into(),
            })
            .await
            .expect("stream should load");
        assert_eq!(state.status, PlayerStatus::Paused);

        let tracks = player.tracks().await.expect("tracks should be available");
        assert!(!tracks.audio.is_empty());
    }
}
