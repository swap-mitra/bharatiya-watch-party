use std::{
    collections::HashMap,
    sync::Arc,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

use app_core::{
    AppError, AppResult, ChatMessage, ClientMessage, CreateRoomRequest, CreateRoomResponse,
    JoinRoomRequest, JoinRoomResponse, MAX_VIEWERS, Participant, ParticipantRole, PlaybackCommand,
    PlayerState, RoomCloseReason, RoomCode, RoomSnapshot, ServerMessage, SessionId, validation,
};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use futures::{sink::SinkExt, stream::StreamExt};
use parking_lot::RwLock;
use serde::Deserialize;
use tokio::sync::mpsc;
use tracing::{info, warn};

const DEFAULT_ROOM_TTL_SECONDS: u64 = 60 * 60 * 4;
const MAX_CHAT_LENGTH: usize = 500;

#[derive(Clone, Debug)]
pub struct ServiceConfig {
    pub room_ttl: Duration,
}

impl Default for ServiceConfig {
    fn default() -> Self {
        Self {
            room_ttl: Duration::from_secs(DEFAULT_ROOM_TTL_SECONDS),
        }
    }
}

#[derive(Clone)]
pub struct AppState {
    registry: Arc<RoomRegistry>,
}

impl AppState {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            registry: Arc::new(RoomRegistry::new(config)),
        }
    }

    pub fn registry(&self) -> Arc<RoomRegistry> {
        Arc::clone(&self.registry)
    }
}

pub fn app_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(healthcheck))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{room_code}/join", post(join_room))
        .route("/ws", get(ws_handler))
        .with_state(state)
}

async fn healthcheck() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

async fn create_room(
    State(state): State<AppState>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, ApiError> {
    Ok(Json(state.registry.create_room(request)?))
}

async fn join_room(
    State(state): State<AppState>,
    Path(room_code): Path<String>,
    Json(request): Json<JoinRoomRequest>,
) -> Result<Json<JoinRoomResponse>, ApiError> {
    let room_code = RoomCode::parse(room_code)?;
    Ok(Json(state.registry.reserve_viewer(room_code, request)?))
}

#[derive(Debug, Deserialize)]
struct WsParams {
    room_code: String,
    session_id: SessionId,
}

async fn ws_handler(
    ws: WebSocketUpgrade,
    State(state): State<AppState>,
    Query(params): Query<WsParams>,
) -> Result<impl IntoResponse, ApiError> {
    let room_code = RoomCode::parse(params.room_code)?;
    Ok(ws.on_upgrade(move |socket| {
        client_session(socket, state.registry(), room_code, params.session_id)
    }))
}

async fn client_session(
    socket: WebSocket,
    registry: Arc<RoomRegistry>,
    room_code: RoomCode,
    session_id: SessionId,
) {
    let (mut sender, mut receiver) = socket.split();
    let (tx, mut rx) = mpsc::unbounded_channel::<ServerMessage>();

    let welcome = match registry.connect(room_code.clone(), session_id, tx) {
        Ok(welcome) => welcome,
        Err(err) => {
            let message = serde_json::to_string(&ServerMessage::Error {
                code: "connect_failed".into(),
                message: err.to_string(),
            })
            .unwrap_or_else(|_| "{\"type\":\"error\"}".into());
            let _ = sender.send(Message::Text(message.into())).await;
            return;
        }
    };

    let initial = serde_json::to_string(&welcome).expect("welcome serializes");
    if sender.send(Message::Text(initial.into())).await.is_err() {
        registry.disconnect(&room_code, &session_id);
        return;
    }

    let room_code_for_sender = room_code.clone();
    let session_for_sender = session_id;
    let registry_for_sender = Arc::clone(&registry);
    let send_task = tokio::spawn(async move {
        while let Some(message) = rx.recv().await {
            match serde_json::to_string(&message) {
                Ok(payload) => {
                    if sender.send(Message::Text(payload.into())).await.is_err() {
                        break;
                    }
                }
                Err(err) => warn!(?err, "failed to serialize outbound message"),
            }
        }
        registry_for_sender.disconnect(&room_code_for_sender, &session_for_sender);
    });

    while let Some(Ok(message)) = receiver.next().await {
        match message {
            Message::Text(text) => match serde_json::from_str::<ClientMessage>(&text) {
                Ok(client_message) => {
                    if let Err(err) =
                        registry.handle_client_message(&room_code, &session_id, client_message)
                    {
                        registry.send_to(
                            &room_code,
                            &session_id,
                            ServerMessage::Error {
                                code: "message_rejected".into(),
                                message: err.to_string(),
                            },
                        );
                    }
                }
                Err(err) => {
                    registry.send_to(
                        &room_code,
                        &session_id,
                        ServerMessage::Error {
                            code: "bad_message".into(),
                            message: err.to_string(),
                        },
                    );
                }
            },
            Message::Close(_) => break,
            Message::Ping(_) | Message::Pong(_) | Message::Binary(_) => {}
        }
    }

    send_task.abort();
    registry.disconnect(&room_code, &session_id);
}

#[derive(Debug)]
pub struct RoomRegistry {
    config: ServiceConfig,
    rooms: RwLock<HashMap<RoomCode, RoomRecord>>,
}

impl RoomRegistry {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            rooms: RwLock::new(HashMap::new()),
        }
    }

    pub fn create_room(&self, request: CreateRoomRequest) -> AppResult<CreateRoomResponse> {
        validation::validate_display_name(&request.display_name)?;

        let mut rooms = self.rooms.write();
        let mut room_code = RoomCode::generate();
        while rooms.contains_key(&room_code) {
            room_code = RoomCode::generate();
        }

        let session_id = SessionId::new();
        let host = ParticipantRecord::new(Participant {
            session_id,
            display_name: request.display_name.trim().to_string(),
            role: ParticipantRole::Host,
            connected: false,
            ready: false,
        });

        rooms.insert(
            room_code.clone(),
            RoomRecord {
                host_session_id: session_id,
                participants: HashMap::from([(session_id, host)]),
                playback: PlayerState::default(),
                last_sequence: 0,
                expires_at_ms: now_ms() + self.config.room_ttl.as_millis() as u64,
            },
        );

        Ok(CreateRoomResponse {
            room_code,
            session_id,
            role: ParticipantRole::Host,
            max_viewers: MAX_VIEWERS,
            expires_in_seconds: self.config.room_ttl.as_secs(),
        })
    }

    pub fn reserve_viewer(
        &self,
        room_code: RoomCode,
        request: JoinRoomRequest,
    ) -> AppResult<JoinRoomResponse> {
        validation::validate_display_name(&request.display_name)?;
        let mut rooms = self.rooms.write();
        self.prune_if_expired_locked(&mut rooms, &room_code);
        let room = rooms.get_mut(&room_code).ok_or(AppError::RoomNotFound)?;

        if room.viewer_count() >= MAX_VIEWERS {
            return Err(AppError::RoomFull);
        }

        if room.participants.values().any(|participant| {
            participant
                .participant
                .display_name
                .eq_ignore_ascii_case(request.display_name.trim())
        }) {
            return Err(AppError::DuplicateParticipant);
        }

        let session_id = SessionId::new();
        room.participants.insert(
            session_id,
            ParticipantRecord::new(Participant {
                session_id,
                display_name: request.display_name.trim().to_string(),
                role: ParticipantRole::Viewer,
                connected: false,
                ready: false,
            }),
        );

        room.touch(self.config.room_ttl);
        let snapshot = room.snapshot(room_code.clone());
        Ok(JoinRoomResponse {
            room_code,
            session_id,
            role: ParticipantRole::Viewer,
            max_viewers: MAX_VIEWERS,
            room: snapshot,
        })
    }

    pub fn connect(
        &self,
        room_code: RoomCode,
        session_id: SessionId,
        sender: mpsc::UnboundedSender<ServerMessage>,
    ) -> AppResult<ServerMessage> {
        let mut rooms = self.rooms.write();
        let room = rooms.get_mut(&room_code).ok_or(AppError::RoomNotFound)?;
        let participant = room
            .participants
            .get_mut(&session_id)
            .ok_or(AppError::ParticipantNotFound)?;
        participant.participant.connected = true;
        participant.sender = Some(sender);
        room.touch(self.config.room_ttl);
        let snapshot = room.snapshot(room_code.clone());
        let playback = room.playback.clone();
        drop(rooms);
        self.broadcast_room_snapshot(&room_code);
        Ok(ServerMessage::Welcome {
            room: snapshot,
            playback,
            self_session_id: session_id,
        })
    }

    pub fn disconnect(&self, room_code: &RoomCode, session_id: &SessionId) {
        let mut rooms = self.rooms.write();
        let mut should_remove = false;
        let mut broadcast_snapshot = None;
        let mut close_targets = Vec::new();

        if let Some(room) = rooms.get_mut(room_code) {
            if let Some(record) = room.participants.get_mut(session_id) {
                record.participant.connected = false;
                record.ready = false;
                record.sender = None;

                if record.participant.role == ParticipantRole::Host {
                    close_targets.extend(room.active_senders_excluding(session_id));
                    should_remove = true;
                } else {
                    room.touch(self.config.room_ttl);
                    broadcast_snapshot = Some(room.snapshot(room_code.clone()));
                }
            }
        }

        if should_remove {
            rooms.remove(room_code);
        }
        drop(rooms);

        if should_remove {
            for sender in close_targets {
                let _ = sender.send(ServerMessage::RoomClosed {
                    reason: RoomCloseReason::HostDisconnected,
                });
            }
        } else if let Some(snapshot) = broadcast_snapshot {
            self.broadcast(room_code, ServerMessage::Presence(snapshot));
        }
    }

    pub fn handle_client_message(
        &self,
        room_code: &RoomCode,
        session_id: &SessionId,
        message: ClientMessage,
    ) -> AppResult<()> {
        match message {
            ClientMessage::Ping => Ok(()),
            ClientMessage::ReadyState { ready } => {
                let snapshot = {
                    let mut rooms = self.rooms.write();
                    let room = rooms.get_mut(room_code).ok_or(AppError::RoomNotFound)?;
                    let participant = room
                        .participants
                        .get_mut(session_id)
                        .ok_or(AppError::ParticipantNotFound)?;
                    participant.ready = ready;
                    participant.participant.ready = ready;
                    room.touch(self.config.room_ttl);
                    room.snapshot(room_code.clone())
                };
                self.broadcast(room_code, ServerMessage::Presence(snapshot));
                Ok(())
            }
            ClientMessage::ChatSend { text } => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.len() > MAX_CHAT_LENGTH {
                    return Err(AppError::Validation(
                        "chat messages must be between 1 and 500 characters".into(),
                    ));
                }

                let chat_message = {
                    let mut rooms = self.rooms.write();
                    let room = rooms.get_mut(room_code).ok_or(AppError::RoomNotFound)?;
                    let sender_display_name = room
                        .participants
                        .get(session_id)
                        .ok_or(AppError::ParticipantNotFound)?
                        .participant
                        .display_name
                        .clone();
                    room.touch(self.config.room_ttl);
                    ChatMessage {
                        id: uuid::Uuid::new_v4().to_string(),
                        sender_session_id: *session_id,
                        sender_display_name,
                        text: trimmed.to_string(),
                        sent_at_ms: now_ms(),
                    }
                };
                self.broadcast(room_code, ServerMessage::Chat(chat_message));
                Ok(())
            }
            ClientMessage::PlaybackCommand(command) => {
                let is_authorized = {
                    let rooms = self.rooms.read();
                    let room = rooms.get(room_code).ok_or(AppError::RoomNotFound)?;
                    room.host_session_id == *session_id
                };
                if !is_authorized {
                    return Err(AppError::Unauthorized);
                }

                if let Some(url) = &command.stream_url {
                    app_core::validation::validate_stream_url(url)?;
                }

                let command = {
                    let mut rooms = self.rooms.write();
                    let room = rooms.get_mut(room_code).ok_or(AppError::RoomNotFound)?;
                    if command.seq <= room.last_sequence {
                        return Ok(());
                    }
                    room.last_sequence = command.seq;
                    room.touch(self.config.room_ttl);
                    room.playback = apply_playback_command(room.playback.clone(), &command);
                    command
                };
                self.broadcast(room_code, ServerMessage::Playback(command));
                Ok(())
            }
        }
    }

    pub fn send_to(&self, room_code: &RoomCode, session_id: &SessionId, message: ServerMessage) {
        let sender = {
            let rooms = self.rooms.read();
            rooms
                .get(room_code)
                .and_then(|room| room.participants.get(session_id))
                .and_then(|record| record.sender.clone())
        };

        if let Some(sender) = sender {
            let _ = sender.send(message);
        }
    }

    pub fn sweep_expired(&self) {
        let mut rooms = self.rooms.write();
        let expired: Vec<_> = rooms
            .iter()
            .filter_map(|(room_code, room)| {
                if room.expires_at_ms <= now_ms() {
                    Some(room_code.clone())
                } else {
                    None
                }
            })
            .collect();

        for room_code in expired {
            if let Some(room) = rooms.remove(&room_code) {
                for sender in room.active_senders() {
                    let _ = sender.send(ServerMessage::RoomClosed {
                        reason: RoomCloseReason::Expired,
                    });
                }
            }
        }
    }

    fn prune_if_expired_locked(
        &self,
        rooms: &mut HashMap<RoomCode, RoomRecord>,
        room_code: &RoomCode,
    ) {
        let expired = rooms
            .get(room_code)
            .map(|room| room.expires_at_ms <= now_ms())
            .unwrap_or(false);
        if expired {
            rooms.remove(room_code);
        }
    }

    fn broadcast_room_snapshot(&self, room_code: &RoomCode) {
        let snapshot = {
            let rooms = self.rooms.read();
            rooms
                .get(room_code)
                .map(|room| room.snapshot(room_code.clone()))
        };
        if let Some(snapshot) = snapshot {
            self.broadcast(room_code, ServerMessage::Presence(snapshot));
        }
    }

    fn broadcast(&self, room_code: &RoomCode, message: ServerMessage) {
        let senders = {
            let rooms = self.rooms.read();
            rooms
                .get(room_code)
                .map(|room| room.active_senders())
                .unwrap_or_default()
        };
        for sender in senders {
            let _ = sender.send(message.clone());
        }
    }
}

#[derive(Debug)]
struct RoomRecord {
    host_session_id: SessionId,
    participants: HashMap<SessionId, ParticipantRecord>,
    playback: PlayerState,
    last_sequence: u64,
    expires_at_ms: u64,
}

impl RoomRecord {
    fn viewer_count(&self) -> usize {
        self.participants
            .values()
            .filter(|participant| participant.participant.role == ParticipantRole::Viewer)
            .count()
    }

    fn snapshot(&self, room_code: RoomCode) -> RoomSnapshot {
        let mut participants = self
            .participants
            .values()
            .map(|participant| participant.participant.clone())
            .collect::<Vec<_>>();
        participants.sort_by(|left, right| left.display_name.cmp(&right.display_name));
        RoomSnapshot {
            room_code,
            host_session_id: self.host_session_id,
            max_viewers: MAX_VIEWERS,
            participants,
        }
    }

    fn touch(&mut self, ttl: Duration) {
        self.expires_at_ms = now_ms() + ttl.as_millis() as u64;
    }

    fn active_senders(&self) -> Vec<mpsc::UnboundedSender<ServerMessage>> {
        self.participants
            .values()
            .filter_map(|participant| participant.sender.clone())
            .collect()
    }

    fn active_senders_excluding(
        &self,
        exclude: &SessionId,
    ) -> Vec<mpsc::UnboundedSender<ServerMessage>> {
        self.participants
            .iter()
            .filter_map(|(session_id, participant)| {
                if session_id == exclude {
                    None
                } else {
                    participant.sender.clone()
                }
            })
            .collect()
    }
}

#[derive(Debug)]
struct ParticipantRecord {
    participant: Participant,
    ready: bool,
    sender: Option<mpsc::UnboundedSender<ServerMessage>>,
}

impl ParticipantRecord {
    fn new(participant: Participant) -> Self {
        Self {
            participant,
            ready: false,
            sender: None,
        }
    }
}

fn apply_playback_command(mut state: PlayerState, command: &PlaybackCommand) -> PlayerState {
    match command.action {
        app_core::PlaybackAction::LoadStream => {
            state.active_source = command.stream_url.clone();
            state.position_ms = 0;
            state.status = app_core::PlayerStatus::Loading;
            state.last_error = None;
        }
        app_core::PlaybackAction::Play => state.status = app_core::PlayerStatus::Playing,
        app_core::PlaybackAction::Pause => state.status = app_core::PlayerStatus::Paused,
        app_core::PlaybackAction::Seek => {
            state.position_ms = command.position_ms.unwrap_or(state.position_ms);
        }
        app_core::PlaybackAction::Stop => {
            state.position_ms = 0;
            state.status = app_core::PlayerStatus::Stopped;
        }
    }
    state
}

fn now_ms() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_millis() as u64
}

#[derive(Debug)]
pub struct ApiError(pub AppError);

impl From<AppError> for ApiError {
    fn from(value: AppError) -> Self {
        Self(value)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let status = match self.0 {
            AppError::Validation(_)
            | AppError::InvalidStreamUrl
            | AppError::DuplicateParticipant => StatusCode::BAD_REQUEST,
            AppError::RoomNotFound => StatusCode::NOT_FOUND,
            AppError::RoomFull => StatusCode::CONFLICT,
            AppError::Unauthorized => StatusCode::FORBIDDEN,
            AppError::RoomClosed => StatusCode::GONE,
            AppError::ParticipantNotFound => StatusCode::UNAUTHORIZED,
            AppError::Transport(_) => StatusCode::INTERNAL_SERVER_ERROR,
        };
        (
            status,
            Json(serde_json::json!({
                "error": self.0.to_string(),
            })),
        )
            .into_response()
    }
}

pub async fn run() -> AppResult<()> {
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let state = AppState::new(ServiceConfig::default());
    let registry = state.registry();
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(60));
        loop {
            interval.tick().await;
            registry.sweep_expired();
        }
    });

    let listener = tokio::net::TcpListener::bind("0.0.0.0:4000")
        .await
        .map_err(|err| AppError::Transport(err.to_string()))?;
    info!("signal-service listening on 0.0.0.0:4000");
    axum::serve(listener, app_router(state))
        .await
        .map_err(|err| AppError::Transport(err.to_string()))
}
