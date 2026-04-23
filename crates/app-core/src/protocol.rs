use serde::{Deserialize, Serialize};

use crate::{
    player::PlayerState,
    room::{Participant, ParticipantRole, RoomCode, SessionId},
};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoomRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateRoomResponse {
    pub room_code: RoomCode,
    pub session_id: SessionId,
    pub role: ParticipantRole,
    pub max_viewers: usize,
    pub expires_in_seconds: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoomRequest {
    pub display_name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct JoinRoomResponse {
    pub room_code: RoomCode,
    pub session_id: SessionId,
    pub role: ParticipantRole,
    pub max_viewers: usize,
    pub room: RoomSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RoomSnapshot {
    pub room_code: RoomCode,
    pub host_session_id: SessionId,
    pub max_viewers: usize,
    pub participants: Vec<Participant>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PlaybackAction {
    LoadStream,
    Play,
    Pause,
    Seek,
    Stop,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PlaybackCommand {
    pub seq: u64,
    pub action: PlaybackAction,
    pub position_ms: Option<u64>,
    pub stream_url: Option<String>,
    pub issued_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ChatMessage {
    pub id: String,
    pub sender_session_id: SessionId,
    pub sender_display_name: String,
    pub text: String,
    pub sent_at_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RoomCloseReason {
    HostDisconnected,
    Expired,
    ClosedByHost,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ClientMessage {
    Ping,
    ReadyState { ready: bool },
    ChatSend { text: String },
    PlaybackCommand(PlaybackCommand),
    CloseRoom,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", content = "payload", rename_all = "snake_case")]
pub enum ServerMessage {
    Welcome {
        room: RoomSnapshot,
        playback: PlayerState,
        self_session_id: SessionId,
    },
    Presence(RoomSnapshot),
    Chat(ChatMessage),
    Playback(PlaybackCommand),
    Error {
        code: String,
        message: String,
    },
    RoomClosed {
        reason: RoomCloseReason,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{PlayerStatus, room::RoomCode};

    #[test]
    fn client_message_round_trip() {
        let message = ClientMessage::PlaybackCommand(PlaybackCommand {
            seq: 3,
            action: PlaybackAction::Seek,
            position_ms: Some(42_000),
            stream_url: None,
            issued_at_ms: 123,
        });

        let encoded = serde_json::to_string(&message).expect("serializes");
        let decoded: ClientMessage = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(message, decoded);
    }

    #[test]
    fn server_message_round_trip() {
        let session_id = SessionId::new();
        let room = RoomSnapshot {
            room_code: RoomCode::parse("abc123").expect("valid room code"),
            host_session_id: session_id,
            max_viewers: 10,
            participants: vec![Participant {
                session_id,
                display_name: "Host".into(),
                role: ParticipantRole::Host,
                connected: true,
                ready: true,
            }],
        };
        let message = ServerMessage::Welcome {
            room,
            playback: PlayerState {
                status: PlayerStatus::Paused,
                active_source: Some("https://example.com/movie.m3u8".into()),
                ..PlayerState::default()
            },
            self_session_id: session_id,
        };

        let encoded = serde_json::to_string(&message).expect("serializes");
        let decoded: ServerMessage = serde_json::from_str(&encoded).expect("deserializes");
        assert_eq!(message, decoded);
    }
}
