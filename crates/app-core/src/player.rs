use async_trait::async_trait;
use serde::{Deserialize, Serialize};

use crate::{AppResult, validation};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlayerStatus {
    Idle,
    Loading,
    Playing,
    Paused,
    Buffering,
    Stopped,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum MediaTrackKind {
    Audio,
    Subtitle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MediaTrack {
    pub id: String,
    pub label: String,
    pub language: Option<String>,
    pub codec: Option<String>,
    pub kind: MediaTrackKind,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct TrackCatalog {
    pub audio: Vec<MediaTrack>,
    pub subtitles: Vec<MediaTrack>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlayerState {
    pub status: PlayerStatus,
    pub active_source: Option<String>,
    pub position_ms: u64,
    pub duration_ms: Option<u64>,
    pub volume: u8,
    pub muted: bool,
    pub selected_audio_track: Option<String>,
    pub selected_subtitle_track: Option<String>,
    pub last_error: Option<String>,
}

impl Default for PlayerState {
    fn default() -> Self {
        Self {
            status: PlayerStatus::Idle,
            active_source: None,
            position_ms: 0,
            duration_ms: None,
            volume: 100,
            muted: false,
            selected_audio_track: None,
            selected_subtitle_track: None,
            last_error: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadStreamRequest {
    pub url: String,
}

impl LoadStreamRequest {
    pub fn validate(&self) -> AppResult<()> {
        validation::validate_stream_url(&self.url)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum PlayerEvent {
    StateChanged(PlayerState),
    TracksChanged(TrackCatalog),
}

#[async_trait]
pub trait PlayerAdapter: Send + Sync {
    async fn load_stream(&self, request: LoadStreamRequest) -> AppResult<PlayerState>;
    async fn play(&self) -> AppResult<PlayerState>;
    async fn pause(&self) -> AppResult<PlayerState>;
    async fn seek(&self, position_ms: u64) -> AppResult<PlayerState>;
    async fn stop(&self) -> AppResult<PlayerState>;
    async fn state(&self) -> AppResult<PlayerState>;
    async fn tracks(&self) -> AppResult<TrackCatalog>;
    async fn select_audio_track(&self, track_id: String) -> AppResult<PlayerState>;
    async fn select_subtitle_track(&self, track_id: Option<String>) -> AppResult<PlayerState>;
}
