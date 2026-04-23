use std::{
    env,
    ffi::{CStr, CString, c_char, c_void},
    ptr::NonNull,
    sync::Arc,
    time::Duration,
};

use app_core::{
    AppError, AppResult, LoadStreamRequest, MediaTrack, MediaTrackKind, PlayerAdapter, PlayerState,
    PlayerStatus, TrackCatalog, validation,
};
use async_trait::async_trait;
use libloading::Library;
use parking_lot::{Mutex, RwLock};
use tauri::{AppHandle, Emitter, State};

const PLAYER_STATE_EVENT: &str = "player:state";
const PLAYER_TRACKS_EVENT: &str = "player:tracks";
const PLAYER_POLL_INTERVAL_MS: u64 = 250;

struct DesktopState {
    player: Arc<MpvAdapter>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum BackendKind {
    Libmpv,
    Mock,
}

struct MpvAdapter {
    backend: PlaybackBackend,
    backend_kind: BackendKind,
    backend_warning: Option<String>,
    state: RwLock<PlayerState>,
    tracks: RwLock<TrackCatalog>,
}

enum PlaybackBackend {
    Real(RealMpvBackend),
    Mock,
}

impl MpvAdapter {
    fn bootstrap() -> Self {
        match RealMpvBackend::new() {
            Ok(backend) => {
                let adapter = Self {
                    backend: PlaybackBackend::Real(backend),
                    backend_kind: BackendKind::Libmpv,
                    backend_warning: None,
                    state: RwLock::new(PlayerState::default()),
                    tracks: RwLock::new(TrackCatalog::default()),
                };
                let _ = adapter.refresh_from_backend();
                adapter
            }
            Err(error) => Self {
                backend: PlaybackBackend::Mock,
                backend_kind: BackendKind::Mock,
                backend_warning: Some(format!(
                    "libmpv was not available, using the development harness instead: {error}"
                )),
                state: RwLock::new(PlayerState::default()),
                tracks: RwLock::new(TrackCatalog::default()),
            },
        }
    }

    #[cfg(test)]
    fn new_mock() -> Self {
        Self {
            backend: PlaybackBackend::Mock,
            backend_kind: BackendKind::Mock,
            backend_warning: None,
            state: RwLock::new(PlayerState::default()),
            tracks: RwLock::new(TrackCatalog::default()),
        }
    }

    fn backend_label(&self) -> &'static str {
        match self.backend_kind {
            BackendKind::Libmpv => "libmpv",
            BackendKind::Mock => "mock",
        }
    }

    fn backend_warning(&self) -> Option<String> {
        self.backend_warning.clone()
    }

    fn refresh_from_backend(&self) -> AppResult<bool> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                let (next_state, next_tracks) = backend.snapshot()?;
                let mut changed = false;

                {
                    let mut state = self.state.write();
                    if *state != next_state {
                        *state = next_state;
                        changed = true;
                    }
                }

                {
                    let mut tracks = self.tracks.write();
                    if *tracks != next_tracks {
                        *tracks = next_tracks;
                        changed = true;
                    }
                }

                Ok(changed)
            }
            PlaybackBackend::Mock => Ok(false),
        }
    }

    fn derive_mock_tracks(url: &str) -> TrackCatalog {
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

    fn set_loading_state(&self, url: &str) {
        let mut state = self.state.write();
        *state = PlayerState {
            status: PlayerStatus::Loading,
            active_source: Some(url.to_string()),
            position_ms: 0,
            duration_ms: None,
            volume: 100,
            muted: false,
            selected_audio_track: None,
            selected_subtitle_track: None,
            last_error: None,
        };
        *self.tracks.write() = TrackCatalog::default();
    }

    fn snapshot_state(&self) -> PlayerState {
        self.state.read().clone()
    }

    fn snapshot_tracks(&self) -> TrackCatalog {
        self.tracks.read().clone()
    }
}

#[async_trait]
impl PlayerAdapter for MpvAdapter {
    async fn load_stream(&self, request: LoadStreamRequest) -> AppResult<PlayerState> {
        request.validate()?;
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                self.set_loading_state(&request.url);
                backend.load_stream(&request.url)?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
                let catalog = Self::derive_mock_tracks(&request.url);
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
        }
    }

    async fn play(&self) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.play()?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
                let mut state = self.state.write();
                if state.active_source.is_none() {
                    return Err(AppError::Validation("load a stream before playing".into()));
                }
                state.status = PlayerStatus::Playing;
                Ok(state.clone())
            }
        }
    }

    async fn pause(&self) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.pause()?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
                let mut state = self.state.write();
                if state.active_source.is_none() {
                    return Err(AppError::Validation("load a stream before pausing".into()));
                }
                state.status = PlayerStatus::Paused;
                Ok(state.clone())
            }
        }
    }

    async fn seek(&self, position_ms: u64) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.seek(position_ms)?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
                let mut state = self.state.write();
                if state.active_source.is_none() {
                    return Err(AppError::Validation("load a stream before seeking".into()));
                }
                state.position_ms = position_ms;
                Ok(state.clone())
            }
        }
    }

    async fn stop(&self) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.stop()?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
                let mut state = self.state.write();
                state.status = PlayerStatus::Stopped;
                state.position_ms = 0;
                Ok(state.clone())
            }
        }
    }

    async fn state(&self) -> AppResult<PlayerState> {
        if matches!(&self.backend, PlaybackBackend::Real(_)) {
            let _ = self.refresh_from_backend();
        }
        Ok(self.snapshot_state())
    }

    async fn tracks(&self) -> AppResult<TrackCatalog> {
        if matches!(&self.backend, PlaybackBackend::Real(_)) {
            let _ = self.refresh_from_backend();
        }
        Ok(self.snapshot_tracks())
    }

    async fn select_audio_track(&self, track_id: String) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.select_audio_track(&track_id)?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
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
        }
    }

    async fn select_subtitle_track(&self, track_id: Option<String>) -> AppResult<PlayerState> {
        match &self.backend {
            PlaybackBackend::Real(backend) => {
                backend.select_subtitle_track(track_id.as_deref())?;
                let _ = self.refresh_from_backend();
                Ok(self.snapshot_state())
            }
            PlaybackBackend::Mock => {
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
    }
}

struct RealMpvBackend {
    client: Mutex<MpvClient>,
    last_loaded_source: Mutex<Option<String>>,
}

impl RealMpvBackend {
    fn new() -> AppResult<Self> {
        let client = MpvClient::new()?;
        Ok(Self {
            client: Mutex::new(client),
            last_loaded_source: Mutex::new(None),
        })
    }

    fn load_stream(&self, url: &str) -> AppResult<()> {
        let client = self.client.lock();
        client.command(&["loadfile", url, "replace"])?;
        client.set_property_string("pause", "yes")?;
        *self.last_loaded_source.lock() = Some(url.to_string());
        Ok(())
    }

    fn play(&self) -> AppResult<()> {
        self.client.lock().set_property_string("pause", "no")
    }

    fn pause(&self) -> AppResult<()> {
        self.client.lock().set_property_string("pause", "yes")
    }

    fn seek(&self, position_ms: u64) -> AppResult<()> {
        let seconds = format!("{:.3}", position_ms as f64 / 1000.0);
        self.client
            .lock()
            .command(&["seek", &seconds, "absolute", "exact"])
    }

    fn stop(&self) -> AppResult<()> {
        self.client.lock().command(&["stop"])
    }

    fn select_audio_track(&self, track_id: &str) -> AppResult<()> {
        self.client.lock().set_property_string("aid", track_id)
    }

    fn select_subtitle_track(&self, track_id: Option<&str>) -> AppResult<()> {
        self.client
            .lock()
            .set_property_string("sid", track_id.unwrap_or("no"))
    }

    fn snapshot(&self) -> AppResult<(PlayerState, TrackCatalog)> {
        let client = self.client.lock();
        let active_source = client.get_property_string("path")?;
        let paused = client.get_property_bool("pause")?.unwrap_or(false);
        let buffering = client
            .get_property_bool("paused-for-cache")?
            .unwrap_or(false);
        let idle = client.get_property_bool("idle-active")?.unwrap_or(false);
        let position_ms = client
            .get_property_f64("time-pos")?
            .map(seconds_to_ms)
            .unwrap_or(0);
        let duration_ms = client.get_property_f64("duration")?.map(seconds_to_ms);
        let volume = client
            .get_property_f64("volume")?
            .map(|value| value.round().clamp(0.0, 100.0) as u8)
            .unwrap_or(100);
        let muted = client.get_property_bool("mute")?.unwrap_or(false);
        let tracks = client.track_catalog()?;
        let selected_audio_track = tracks
            .audio
            .iter()
            .find(|track| track.selected)
            .map(|track| track.id.clone());
        let selected_subtitle_track = tracks
            .subtitles
            .iter()
            .find(|track| track.selected)
            .map(|track| track.id.clone());
        let status = if active_source.is_none() {
            if idle && self.last_loaded_source.lock().is_some() {
                PlayerStatus::Stopped
            } else {
                PlayerStatus::Idle
            }
        } else if buffering {
            PlayerStatus::Buffering
        } else if paused {
            PlayerStatus::Paused
        } else {
            PlayerStatus::Playing
        };

        Ok((
            PlayerState {
                status,
                active_source,
                position_ms,
                duration_ms,
                volume,
                muted,
                selected_audio_track,
                selected_subtitle_track,
                last_error: None,
            },
            tracks,
        ))
    }
}

struct MpvClient {
    _library: Library,
    symbols: MpvSymbols,
    handle: NonNull<c_void>,
}

// libmpv access is serialized through RealMpvBackend::client, so moving the client across
// threads is safe as long as callers do not access it without the mutex.
unsafe impl Send for MpvClient {}

impl MpvClient {
    fn new() -> AppResult<Self> {
        let library = load_mpv_library()?;
        let symbols = unsafe { MpvSymbols::load(&library)? };
        let handle = unsafe { (symbols.create)() };
        let handle = NonNull::new(handle)
            .ok_or_else(|| AppError::Transport("libmpv returned a null player handle".into()))?;
        let client = Self {
            _library: library,
            symbols,
            handle,
        };
        client.configure()?;
        Ok(client)
    }

    fn configure(&self) -> AppResult<()> {
        self.set_option_string("terminal", "no")?;
        self.set_option_string("input-default-bindings", "yes")?;
        self.set_option_string("input-vo-keyboard", "yes")?;
        self.set_option_string("osc", "yes")?;
        self.set_option_string("keep-open", "yes")?;
        self.set_option_string("idle", "yes")?;
        self.set_option_string("force-window", "yes")?;
        self.set_option_string("pause", "yes")?;
        self.check_error(unsafe { (self.symbols.initialize)(self.handle.as_ptr()) })
    }

    fn command(&self, args: &[&str]) -> AppResult<()> {
        let cstrings = args
            .iter()
            .map(|arg| CString::new(*arg).map_err(|err| AppError::Transport(err.to_string())))
            .collect::<Result<Vec<_>, _>>()?;
        let mut pointers = cstrings.iter().map(|arg| arg.as_ptr()).collect::<Vec<_>>();
        pointers.push(std::ptr::null());
        self.check_error(unsafe { (self.symbols.command)(self.handle.as_ptr(), pointers.as_ptr()) })
    }

    fn set_option_string(&self, name: &str, value: &str) -> AppResult<()> {
        let name = CString::new(name).map_err(|err| AppError::Transport(err.to_string()))?;
        let value = CString::new(value).map_err(|err| AppError::Transport(err.to_string()))?;
        self.check_error(unsafe {
            (self.symbols.set_option_string)(self.handle.as_ptr(), name.as_ptr(), value.as_ptr())
        })
    }

    fn set_property_string(&self, name: &str, value: &str) -> AppResult<()> {
        let name = CString::new(name).map_err(|err| AppError::Transport(err.to_string()))?;
        let value = CString::new(value).map_err(|err| AppError::Transport(err.to_string()))?;
        self.check_error(unsafe {
            (self.symbols.set_property_string)(self.handle.as_ptr(), name.as_ptr(), value.as_ptr())
        })
    }

    fn get_property_string(&self, name: &str) -> AppResult<Option<String>> {
        let name = CString::new(name).map_err(|err| AppError::Transport(err.to_string()))?;
        let pointer =
            unsafe { (self.symbols.get_property_string)(self.handle.as_ptr(), name.as_ptr()) };
        if pointer.is_null() {
            return Ok(None);
        }

        let value = unsafe { CStr::from_ptr(pointer) }
            .to_string_lossy()
            .trim()
            .to_string();
        unsafe { (self.symbols.free)(pointer.cast()) };

        if value.is_empty() || value == "no" {
            Ok(None)
        } else {
            Ok(Some(value))
        }
    }

    fn get_property_bool(&self, name: &str) -> AppResult<Option<bool>> {
        Ok(self
            .get_property_string(name)?
            .map(|value| parse_mpv_bool(&value)))
    }

    fn get_property_f64(&self, name: &str) -> AppResult<Option<f64>> {
        Ok(self
            .get_property_string(name)?
            .and_then(|value| value.parse::<f64>().ok()))
    }

    fn track_catalog(&self) -> AppResult<TrackCatalog> {
        let count = self
            .get_property_string("track-list/count")?
            .and_then(|value| value.parse::<usize>().ok())
            .unwrap_or(0);
        let mut audio = Vec::new();
        let mut subtitles = Vec::new();

        for index in 0..count {
            let kind = match self
                .get_property_string(&format!("track-list/{index}/type"))?
                .as_deref()
            {
                Some("audio") => MediaTrackKind::Audio,
                Some("sub") => MediaTrackKind::Subtitle,
                _ => continue,
            };

            let Some(id) = self.get_property_string(&format!("track-list/{index}/id"))? else {
                continue;
            };
            let title = self.get_property_string(&format!("track-list/{index}/title"))?;
            let language = self.get_property_string(&format!("track-list/{index}/lang"))?;
            let codec = self.get_property_string(&format!("track-list/{index}/codec"))?;
            let selected = self
                .get_property_bool(&format!("track-list/{index}/selected"))?
                .unwrap_or(false);
            let label = title.unwrap_or_else(|| build_track_label(kind, language.as_deref(), &id));
            let track = MediaTrack {
                id,
                label,
                language,
                codec,
                kind,
                selected,
            };

            match kind {
                MediaTrackKind::Audio => audio.push(track),
                MediaTrackKind::Subtitle => subtitles.push(track),
            }
        }

        Ok(TrackCatalog { audio, subtitles })
    }

    fn check_error(&self, code: i32) -> AppResult<()> {
        if code >= 0 {
            return Ok(());
        }

        let message = unsafe {
            let ptr = (self.symbols.error_string)(code);
            if ptr.is_null() {
                format!("libmpv error {code}")
            } else {
                CStr::from_ptr(ptr).to_string_lossy().to_string()
            }
        };
        Err(AppError::Transport(format!("libmpv: {message}")))
    }
}

impl Drop for MpvClient {
    fn drop(&mut self) {
        unsafe {
            (self.symbols.terminate_destroy)(self.handle.as_ptr());
        }
    }
}

#[derive(Clone, Copy)]
struct MpvSymbols {
    create: unsafe extern "C" fn() -> *mut c_void,
    initialize: unsafe extern "C" fn(*mut c_void) -> i32,
    terminate_destroy: unsafe extern "C" fn(*mut c_void),
    command: unsafe extern "C" fn(*mut c_void, *const *const c_char) -> i32,
    set_option_string: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> i32,
    set_property_string: unsafe extern "C" fn(*mut c_void, *const c_char, *const c_char) -> i32,
    get_property_string: unsafe extern "C" fn(*mut c_void, *const c_char) -> *mut c_char,
    free: unsafe extern "C" fn(*mut c_void),
    error_string: unsafe extern "C" fn(i32) -> *const c_char,
}

impl MpvSymbols {
    unsafe fn load(library: &Library) -> AppResult<Self> {
        Ok(Self {
            create: *unsafe { library.get(b"mpv_create\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            initialize: *unsafe { library.get(b"mpv_initialize\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            terminate_destroy: *unsafe { library.get(b"mpv_terminate_destroy\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            command: *unsafe { library.get(b"mpv_command\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            set_option_string: *unsafe { library.get(b"mpv_set_option_string\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            set_property_string: *unsafe { library.get(b"mpv_set_property_string\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            get_property_string: *unsafe { library.get(b"mpv_get_property_string\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            free: *unsafe { library.get(b"mpv_free\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
            error_string: *unsafe { library.get(b"mpv_error_string\0") }
                .map_err(|err| AppError::Transport(err.to_string()))?,
        })
    }
}

fn load_mpv_library() -> AppResult<Library> {
    if let Ok(path) = env::var("MPV_LIBRARY_PATH") {
        return unsafe { Library::new(path) }.map_err(|err| AppError::Transport(err.to_string()));
    }

    let candidates = if cfg!(target_os = "windows") {
        vec!["mpv-2.dll", "libmpv-2.dll", "mpv-1.dll"]
    } else if cfg!(target_os = "macos") {
        vec!["libmpv.2.dylib", "libmpv.dylib"]
    } else {
        vec!["libmpv.so.2", "libmpv.so"]
    };

    let mut last_error = None;
    for candidate in candidates {
        match unsafe { Library::new(candidate) } {
            Ok(library) => return Ok(library),
            Err(error) => last_error = Some(error.to_string()),
        }
    }

    Err(AppError::Transport(last_error.unwrap_or_else(|| {
        "libmpv library could not be loaded".into()
    })))
}

fn build_track_label(kind: MediaTrackKind, language: Option<&str>, id: &str) -> String {
    let prefix = match kind {
        MediaTrackKind::Audio => "Audio",
        MediaTrackKind::Subtitle => "Subtitle",
    };
    match language {
        Some(language) => format!("{prefix} {language} ({id})"),
        None => format!("{prefix} {id}"),
    }
}

fn parse_mpv_bool(value: &str) -> bool {
    matches!(value, "yes" | "true" | "1")
}

fn seconds_to_ms(value: f64) -> u64 {
    (value.max(0.0) * 1000.0).round() as u64
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct BootstrapPayload {
    state: PlayerState,
    tracks: TrackCatalog,
    backend: String,
    backend_warning: Option<String>,
}

#[tauri::command]
async fn bootstrap_player(state: State<'_, DesktopState>) -> Result<BootstrapPayload, String> {
    let player = Arc::clone(&state.inner().player);
    Ok(BootstrapPayload {
        state: player.state().await.map_err(to_string_error)?,
        tracks: player.tracks().await.map_err(to_string_error)?,
        backend: player.backend_label().to_string(),
        backend_warning: player.backend_warning(),
    })
}

#[tauri::command]
async fn load_stream(
    app: AppHandle,
    state: State<'_, DesktopState>,
    url: String,
) -> Result<PlayerState, String> {
    validation::validate_stream_url(&url).map_err(to_string_error)?;
    let player = Arc::clone(&state.inner().player);
    let next_state = player
        .load_stream(LoadStreamRequest { url })
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn play(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player.play().await.map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn pause(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player.pause().await.map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn seek(
    app: AppHandle,
    state: State<'_, DesktopState>,
    position_ms: u64,
) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player.seek(position_ms).await.map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn stop(app: AppHandle, state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player.stop().await.map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn player_state(state: State<'_, DesktopState>) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    player.state().await.map_err(to_string_error)
}

#[tauri::command]
async fn player_tracks(state: State<'_, DesktopState>) -> Result<TrackCatalog, String> {
    let player = Arc::clone(&state.inner().player);
    player.tracks().await.map_err(to_string_error)
}

#[tauri::command]
async fn select_audio_track(
    app: AppHandle,
    state: State<'_, DesktopState>,
    track_id: String,
) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player
        .select_audio_track(track_id)
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

#[tauri::command]
async fn select_subtitle_track(
    app: AppHandle,
    state: State<'_, DesktopState>,
    track_id: Option<String>,
) -> Result<PlayerState, String> {
    let player = Arc::clone(&state.inner().player);
    let next_state = player
        .select_subtitle_track(track_id)
        .await
        .map_err(to_string_error)?;
    emit_snapshot(&app, &player).map_err(to_string_error)?;
    Ok(next_state)
}

fn emit_snapshot(app: &AppHandle, player: &Arc<MpvAdapter>) -> AppResult<()> {
    let state = player.snapshot_state();
    let tracks = player.snapshot_tracks();
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
    let player = Arc::new(MpvAdapter::bootstrap());
    tauri::Builder::default()
        .manage(DesktopState {
            player: Arc::clone(&player),
        })
        .setup(move |app| {
            let app_handle = app.handle().clone();
            let player = Arc::clone(&player);
            tauri::async_runtime::spawn(async move {
                let mut ticker =
                    tokio::time::interval(Duration::from_millis(PLAYER_POLL_INTERVAL_MS));
                loop {
                    ticker.tick().await;
                    match player.refresh_from_backend() {
                        Ok(true) => {
                            let _ = emit_snapshot(&app_handle, &player);
                        }
                        Ok(false) => {}
                        Err(_) => {}
                    }
                }
            });
            Ok(())
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
        let player = MpvAdapter::new_mock();
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
