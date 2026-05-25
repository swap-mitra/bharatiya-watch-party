use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    sync::atomic::{AtomicU64, Ordering},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use app_core::{
    AppError, AppResult, ChatMessage, ClientMessage, CreateRoomRequest, CreateRoomResponse,
    JoinRoomRequest, JoinRoomResponse, MAX_VIEWERS, Participant, ParticipantRole, PlaybackCommand,
    PlaybackHeartbeat, PlayerState, RoomCloseReason, RoomCode, RoomSnapshot, ServerMessage,
    SessionId, validation,
};
use axum::{
    Json, Router,
    extract::{
        Path, Query, State,
        ws::{Message, WebSocket, WebSocketUpgrade},
    },
    http::{HeaderValue, Method, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use futures::{sink::SinkExt, stream::StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use tokio::sync::mpsc;
use tower_http::cors::{Any, CorsLayer};
use tracing::{info, warn};

const DEFAULT_ROOM_TTL_SECONDS: u64 = 60 * 60 * 4;
const MAX_CHAT_LENGTH: usize = 500;
const MAX_CHAT_ID_LENGTH: usize = 128;
const CHAT_HISTORY_LIMIT: usize = 50;
const RECENT_CHAT_ID_LIMIT: usize = 200;

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

#[derive(Debug, Default)]
pub struct ServiceMetrics {
    room_create_count: AtomicU64,
    room_join_count: AtomicU64,
    websocket_connect_count: AtomicU64,
    reconnect_count: AtomicU64,
    disconnect_count: AtomicU64,
    room_close_count: AtomicU64,
    room_expiration_count: AtomicU64,
    playback_command_count: AtomicU64,
    playback_heartbeat_count: AtomicU64,
    chat_message_count: AtomicU64,
    unauthorized_message_count: AtomicU64,
    validation_failure_count: AtomicU64,
    stream_validation_failure_count: AtomicU64,
    outbound_message_count: AtomicU64,
    outbound_send_failure_count: AtomicU64,
    playback_fanout_count: AtomicU64,
    playback_fanout_total_ms: AtomicU64,
    playback_fanout_max_ms: AtomicU64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceMetricsSnapshot {
    pub room_create_count: u64,
    pub room_join_count: u64,
    pub active_room_count: u64,
    pub active_participant_count: u64,
    pub websocket_connect_count: u64,
    pub reconnect_count: u64,
    pub disconnect_count: u64,
    pub room_close_count: u64,
    pub room_expiration_count: u64,
    pub playback_command_count: u64,
    pub playback_heartbeat_count: u64,
    pub chat_message_count: u64,
    pub unauthorized_message_count: u64,
    pub validation_failure_count: u64,
    pub stream_validation_failure_count: u64,
    pub outbound_message_count: u64,
    pub outbound_send_failure_count: u64,
    pub playback_fanout_count: u64,
    pub playback_fanout_total_ms: u64,
    pub playback_fanout_max_ms: u64,
}

impl ServiceMetrics {
    fn increment(counter: &AtomicU64) {
        counter.fetch_add(1, Ordering::Relaxed);
    }

    fn add(counter: &AtomicU64, value: u64) {
        counter.fetch_add(value, Ordering::Relaxed);
    }

    fn record_max(counter: &AtomicU64, value: u64) {
        let mut current = counter.load(Ordering::Relaxed);
        while value > current {
            match counter.compare_exchange(current, value, Ordering::Relaxed, Ordering::Relaxed) {
                Ok(_) => break,
                Err(next_current) => current = next_current,
            }
        }
    }

    fn snapshot(
        &self,
        active_room_count: u64,
        active_participant_count: u64,
    ) -> ServiceMetricsSnapshot {
        ServiceMetricsSnapshot {
            room_create_count: self.room_create_count.load(Ordering::Relaxed),
            room_join_count: self.room_join_count.load(Ordering::Relaxed),
            active_room_count,
            active_participant_count,
            websocket_connect_count: self.websocket_connect_count.load(Ordering::Relaxed),
            reconnect_count: self.reconnect_count.load(Ordering::Relaxed),
            disconnect_count: self.disconnect_count.load(Ordering::Relaxed),
            room_close_count: self.room_close_count.load(Ordering::Relaxed),
            room_expiration_count: self.room_expiration_count.load(Ordering::Relaxed),
            playback_command_count: self.playback_command_count.load(Ordering::Relaxed),
            playback_heartbeat_count: self.playback_heartbeat_count.load(Ordering::Relaxed),
            chat_message_count: self.chat_message_count.load(Ordering::Relaxed),
            unauthorized_message_count: self.unauthorized_message_count.load(Ordering::Relaxed),
            validation_failure_count: self.validation_failure_count.load(Ordering::Relaxed),
            stream_validation_failure_count: self
                .stream_validation_failure_count
                .load(Ordering::Relaxed),
            outbound_message_count: self.outbound_message_count.load(Ordering::Relaxed),
            outbound_send_failure_count: self.outbound_send_failure_count.load(Ordering::Relaxed),
            playback_fanout_count: self.playback_fanout_count.load(Ordering::Relaxed),
            playback_fanout_total_ms: self.playback_fanout_total_ms.load(Ordering::Relaxed),
            playback_fanout_max_ms: self.playback_fanout_max_ms.load(Ordering::Relaxed),
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
    let cors = CorsLayer::new()
        .allow_origin([
            HeaderValue::from_static("http://localhost:1420"),
            HeaderValue::from_static("http://127.0.0.1:1420"),
            HeaderValue::from_static("http://tauri.localhost"),
            HeaderValue::from_static("tauri://localhost"),
            HeaderValue::from_static("https://tauri.localhost"),
        ])
        .allow_methods([Method::GET, Method::POST, Method::OPTIONS])
        .allow_headers(Any);

    Router::new()
        .route("/health", get(healthcheck))
        .route("/metrics", get(metrics))
        .route("/networking", get(networking))
        .route("/api/rooms", post(create_room))
        .route("/api/rooms/{room_code}/join", post(join_room))
        .route("/ws", get(ws_handler))
        .layer(cors)
        .with_state(state)
}

async fn healthcheck() -> impl IntoResponse {
    Json(serde_json::json!({ "ok": true }))
}

async fn metrics(State(state): State<AppState>) -> impl IntoResponse {
    Json(state.registry.metrics_snapshot())
}

async fn networking() -> impl IntoResponse {
    Json(NetworkingSnapshot {
        signaling_transport: "websocket",
        media_transport: "direct-client-fetch",
        webrtc_enabled: false,
        stun_configured: false,
        turn_configured: false,
        fallback_transport: "hosted-websocket-signaling",
    })
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NetworkingSnapshot {
    pub signaling_transport: &'static str,
    pub media_transport: &'static str,
    pub webrtc_enabled: bool,
    pub stun_configured: bool,
    pub turn_configured: bool,
    pub fallback_transport: &'static str,
}

async fn create_room(
    State(state): State<AppState>,
    Json(request): Json<CreateRoomRequest>,
) -> Result<Json<CreateRoomResponse>, ApiError> {
    let started_at = Instant::now();
    let response = state.registry.create_room(request)?;
    info!(
        room_code = %response.room_code,
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "http_room_create_completed"
    );
    Ok(Json(response))
}

async fn join_room(
    State(state): State<AppState>,
    Path(room_code): Path<String>,
    Json(request): Json<JoinRoomRequest>,
) -> Result<Json<JoinRoomResponse>, ApiError> {
    let started_at = Instant::now();
    let room_code = RoomCode::parse(room_code)?;
    let response = state.registry.reserve_viewer(room_code, request)?;
    info!(
        room_code = %response.room_code,
        session_id = %response.session_id,
        elapsed_ms = started_at.elapsed().as_millis() as u64,
        "http_room_join_completed"
    );
    Ok(Json(response))
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
            let message = serde_json::to_string(
                &(ServerMessage::Error {
                    code: "connect_failed".into(),
                    message: err.to_string(),
                }),
            )
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
            Message::Close(_) => {
                break;
            }
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
    metrics: ServiceMetrics,
}

impl RoomRegistry {
    pub fn new(config: ServiceConfig) -> Self {
        Self {
            config,
            rooms: RwLock::new(HashMap::new()),
            metrics: ServiceMetrics::default(),
        }
    }

    pub fn metrics_snapshot(&self) -> ServiceMetricsSnapshot {
        let rooms = self.rooms.read();
        let active_room_count = rooms.len() as u64;
        let active_participant_count = rooms
            .values()
            .flat_map(|room| room.participants.values())
            .filter(|participant| participant.participant.connected)
            .count() as u64;
        self.metrics
            .snapshot(active_room_count, active_participant_count)
    }

    pub fn create_room(&self, request: CreateRoomRequest) -> AppResult<CreateRoomResponse> {
        if let Err(err) = validation::validate_display_name(&request.display_name) {
            ServiceMetrics::increment(&self.metrics.validation_failure_count);
            return Err(err);
        }

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
                expires_at_ms: now_ms() + (self.config.room_ttl.as_millis() as u64),
                chat_history: VecDeque::new(),
                recent_chat_ids: VecDeque::new(),
            },
        );

        ServiceMetrics::increment(&self.metrics.room_create_count);
        info!(
            room_code = %room_code,
            session_id = %session_id,
            "room_created"
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
        if let Err(err) = validation::validate_display_name(&request.display_name) {
            ServiceMetrics::increment(&self.metrics.validation_failure_count);
            return Err(err);
        }
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
        ServiceMetrics::increment(&self.metrics.room_join_count);
        info!(
            room_code = %room_code,
            session_id = %session_id,
            viewer_count = room.viewer_count(),
            "viewer_reserved"
        );
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
        let is_reconnect = participant.connection_count > 0;
        participant.participant.connected = true;
        participant.sender = Some(sender);
        participant.connection_count += 1;
        room.touch(self.config.room_ttl);
        let snapshot = room.snapshot(room_code.clone());
        let playback = room.playback.clone();
        let chat_history = room.chat_history();
        drop(rooms);
        ServiceMetrics::increment(&self.metrics.websocket_connect_count);
        if is_reconnect {
            ServiceMetrics::increment(&self.metrics.reconnect_count);
        }
        info!(
            room_code = %room_code,
            session_id = %session_id,
            reconnect = is_reconnect,
            "session_connected"
        );
        self.broadcast_room_snapshot(&room_code);
        Ok(ServerMessage::Welcome {
            room: snapshot,
            playback,
            self_session_id: session_id,
            chat_history,
        })
    }

    pub fn disconnect(&self, room_code: &RoomCode, session_id: &SessionId) {
        let mut rooms = self.rooms.write();
        let mut should_remove = false;
        let mut broadcast_snapshot = None;
        let mut close_targets = Vec::new();

        if let Some(room) = rooms.get_mut(room_code) {
            if let Some(record) = room.participants.get_mut(session_id) {
                let was_connected = record.participant.connected || record.sender.is_some();
                record.participant.connected = false;
                record.ready = false;
                record.sender = None;
                if was_connected {
                    ServiceMetrics::increment(&self.metrics.disconnect_count);
                    info!(
                        room_code = %room_code,
                        session_id = %session_id,
                        role = ?record.participant.role,
                        "session_disconnected"
                    );
                }

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
            ServiceMetrics::increment(&self.metrics.room_close_count);
            info!(
                room_code = %room_code,
                session_id = %session_id,
                reason = ?RoomCloseReason::HostDisconnected,
                "room_closed"
            );
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
            ClientMessage::CloseRoom => self.close_room(room_code, session_id),
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
            ClientMessage::ChatSend { id, text } => {
                let trimmed = text.trim();
                if trimmed.is_empty() || trimmed.len() > MAX_CHAT_LENGTH {
                    ServiceMetrics::increment(&self.metrics.validation_failure_count);
                    return Err(AppError::Validation(
                        "chat messages must be between 1 and 500 characters".into(),
                    ));
                }
                if id.trim().is_empty() || id.len() > MAX_CHAT_ID_LENGTH {
                    ServiceMetrics::increment(&self.metrics.validation_failure_count);
                    return Err(AppError::Validation(
                        "chat message id must be between 1 and 128 characters".into(),
                    ));
                }

                let chat_message = {
                    let mut rooms = self.rooms.write();
                    let room = rooms.get_mut(room_code).ok_or(AppError::RoomNotFound)?;
                    if room.has_recent_chat_id(&id) {
                        return Ok(());
                    }
                    let sender_display_name = room
                        .participants
                        .get(session_id)
                        .ok_or(AppError::ParticipantNotFound)?
                        .participant
                        .display_name
                        .clone();
                    room.touch(self.config.room_ttl);
                    let chat_message = ChatMessage {
                        id,
                        sender_session_id: *session_id,
                        sender_display_name,
                        text: trimmed.to_string(),
                        sent_at_ms: now_ms(),
                    };
                    room.remember_chat_message(chat_message.clone());
                    chat_message
                };
                ServiceMetrics::increment(&self.metrics.chat_message_count);
                info!(
                    room_code = %room_code,
                    session_id = %session_id,
                    chat_id = %chat_message.id,
                    "chat_message_accepted"
                );
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
                    ServiceMetrics::increment(&self.metrics.unauthorized_message_count);
                    return Err(AppError::Unauthorized);
                }

                if let Some(url) = &command.stream_url {
                    if let Err(err) = app_core::validation::validate_stream_url(url) {
                        ServiceMetrics::increment(&self.metrics.stream_validation_failure_count);
                        ServiceMetrics::increment(&self.metrics.validation_failure_count);
                        return Err(err);
                    }
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
                ServiceMetrics::increment(&self.metrics.playback_command_count);
                info!(
                    room_code = %room_code,
                    session_id = %session_id,
                    seq = command.seq,
                    action = ?command.action,
                    "playback_command_accepted"
                );
                self.broadcast(room_code, ServerMessage::Playback(command));
                Ok(())
            }
            ClientMessage::PlaybackHeartbeat(heartbeat) => {
                let is_authorized = {
                    let rooms = self.rooms.read();
                    let room = rooms.get(room_code).ok_or(AppError::RoomNotFound)?;
                    room.host_session_id == *session_id
                };
                if !is_authorized {
                    ServiceMetrics::increment(&self.metrics.unauthorized_message_count);
                    return Err(AppError::Unauthorized);
                }

                if let Some(url) = &heartbeat.active_source {
                    if let Err(err) = app_core::validation::validate_stream_url(url) {
                        ServiceMetrics::increment(&self.metrics.stream_validation_failure_count);
                        ServiceMetrics::increment(&self.metrics.validation_failure_count);
                        return Err(err);
                    }
                }

                let heartbeat = {
                    let mut rooms = self.rooms.write();
                    let room = rooms.get_mut(room_code).ok_or(AppError::RoomNotFound)?;
                    if heartbeat.command_seq < room.last_sequence {
                        return Ok(());
                    }
                    if heartbeat.command_seq > room.last_sequence {
                        ServiceMetrics::increment(&self.metrics.validation_failure_count);
                        return Err(AppError::Validation(
                            "heartbeat cannot advance playback sequence".into(),
                        ));
                    }
                    if room.playback.active_source != heartbeat.active_source {
                        ServiceMetrics::increment(&self.metrics.validation_failure_count);
                        return Err(AppError::Validation(
                            "heartbeat source must match active room source".into(),
                        ));
                    }
                    room.touch(self.config.room_ttl);
                    room.playback = apply_playback_heartbeat(room.playback.clone(), &heartbeat);
                    heartbeat
                };
                ServiceMetrics::increment(&self.metrics.playback_heartbeat_count);
                self.broadcast(room_code, ServerMessage::PlaybackHeartbeat(heartbeat));
                Ok(())
            }
        }
    }

    pub fn close_room(&self, room_code: &RoomCode, session_id: &SessionId) -> AppResult<()> {
        let senders = {
            let rooms = self.rooms.write();
            let room = rooms.get(room_code).ok_or(AppError::RoomNotFound)?;
            if room.host_session_id != *session_id {
                ServiceMetrics::increment(&self.metrics.unauthorized_message_count);
                return Err(AppError::Unauthorized);
            }
            room.active_senders()
        };
        {
            let mut rooms = self.rooms.write();
            rooms.remove(room_code);
        }
        ServiceMetrics::increment(&self.metrics.room_close_count);
        info!(
            room_code = %room_code,
            session_id = %session_id,
            reason = ?RoomCloseReason::ClosedByHost,
            "room_closed"
        );

        for sender in senders {
            let _ = sender.send(ServerMessage::RoomClosed {
                reason: RoomCloseReason::ClosedByHost,
            });
        }

        Ok(())
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
            ServiceMetrics::increment(&self.metrics.outbound_message_count);
            if sender.send(message).is_err() {
                ServiceMetrics::increment(&self.metrics.outbound_send_failure_count);
            }
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
                ServiceMetrics::increment(&self.metrics.room_expiration_count);
                ServiceMetrics::increment(&self.metrics.room_close_count);
                info!(
                    room_code = %room_code,
                    reason = ?RoomCloseReason::Expired,
                    "room_closed"
                );
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
        let started_at = Instant::now();
        let is_playback_fanout = matches!(
            &message,
            ServerMessage::Playback(_) | ServerMessage::PlaybackHeartbeat(_)
        );
        let senders = {
            let rooms = self.rooms.read();
            rooms
                .get(room_code)
                .map(|room| room.active_senders())
                .unwrap_or_default()
        };
        let recipient_count = senders.len() as u64;
        let mut failed_sends = 0_u64;
        for sender in senders {
            if sender.send(message.clone()).is_err() {
                failed_sends += 1;
            }
        }

        ServiceMetrics::add(&self.metrics.outbound_message_count, recipient_count);
        if failed_sends > 0 {
            ServiceMetrics::add(&self.metrics.outbound_send_failure_count, failed_sends);
        }
        if is_playback_fanout {
            let elapsed_ms = started_at.elapsed().as_millis() as u64;
            ServiceMetrics::increment(&self.metrics.playback_fanout_count);
            ServiceMetrics::add(&self.metrics.playback_fanout_total_ms, elapsed_ms);
            ServiceMetrics::record_max(&self.metrics.playback_fanout_max_ms, elapsed_ms);
            info!(
                room_code = %room_code,
                recipient_count,
                failed_sends,
                elapsed_ms,
                "playback_fanout_completed"
            );
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
    chat_history: VecDeque<ChatMessage>,
    recent_chat_ids: VecDeque<String>,
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
        self.expires_at_ms = now_ms() + (ttl.as_millis() as u64);
    }

    fn chat_history(&self) -> Vec<ChatMessage> {
        self.chat_history.iter().cloned().collect()
    }

    fn has_recent_chat_id(&self, id: &str) -> bool {
        self.recent_chat_ids
            .iter()
            .any(|existing_id| existing_id == id)
    }

    fn remember_chat_message(&mut self, message: ChatMessage) {
        self.recent_chat_ids.push_back(message.id.clone());
        while self.recent_chat_ids.len() > RECENT_CHAT_ID_LIMIT {
            self.recent_chat_ids.pop_front();
        }

        self.chat_history.push_back(message);
        while self.chat_history.len() > CHAT_HISTORY_LIMIT {
            self.chat_history.pop_front();
        }
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
    connection_count: u64,
}

impl ParticipantRecord {
    fn new(participant: Participant) -> Self {
        Self {
            participant,
            ready: false,
            sender: None,
            connection_count: 0,
        }
    }
}

fn apply_playback_command(mut state: PlayerState, command: &PlaybackCommand) -> PlayerState {
    match command.action {
        app_core::PlaybackAction::LoadStream => {
            state.active_source = command.stream_url.clone();
            state.position_ms = 0;
            state.playback_rate_percent = 100;
            state.status = app_core::PlayerStatus::Loading;
            state.last_error = None;
        }
        app_core::PlaybackAction::Play => {
            state.playback_rate_percent = 100;
            state.status = app_core::PlayerStatus::Playing;
        }
        app_core::PlaybackAction::Pause => {
            state.playback_rate_percent = 100;
            state.status = app_core::PlayerStatus::Paused;
        }
        app_core::PlaybackAction::Seek => {
            state.playback_rate_percent = 100;
            state.position_ms = command.position_ms.unwrap_or(state.position_ms);
        }
        app_core::PlaybackAction::Stop => {
            state.position_ms = 0;
            state.playback_rate_percent = 100;
            state.status = app_core::PlayerStatus::Stopped;
        }
    }
    state
}

fn apply_playback_heartbeat(mut state: PlayerState, heartbeat: &PlaybackHeartbeat) -> PlayerState {
    state.position_ms = heartbeat.position_ms;
    state.status = heartbeat.status.clone();
    state.active_source = heartbeat.active_source.clone();
    state.last_error = None;
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
