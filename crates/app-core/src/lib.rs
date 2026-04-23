pub mod error;
pub mod player;
pub mod protocol;
pub mod room;
pub mod validation;

pub use error::{AppError, AppResult};
pub use player::{
    LoadStreamRequest, MediaTrack, MediaTrackKind, PlayerAdapter, PlayerEvent, PlayerState,
    PlayerStatus, TrackCatalog,
};
pub use protocol::{
    ChatMessage, ClientMessage, CreateRoomRequest, CreateRoomResponse, JoinRoomRequest,
    JoinRoomResponse, PlaybackAction, PlaybackCommand, RoomCloseReason, RoomSnapshot,
    ServerMessage,
};
pub use room::{MAX_VIEWERS, Participant, ParticipantRole, RoomCode, SessionId};
