use thiserror::Error;

#[derive(Debug, Error, Clone, PartialEq, Eq)]
pub enum AppError {
    #[error("validation failed: {0}")]
    Validation(String),
    #[error("room not found")]
    RoomNotFound,
    #[error("room is full")]
    RoomFull,
    #[error("room is closed")]
    RoomClosed,
    #[error("unauthorized action")]
    Unauthorized,
    #[error("participant not found")]
    ParticipantNotFound,
    #[error("duplicate participant")]
    DuplicateParticipant,
    #[error("invalid stream url")]
    InvalidStreamUrl,
    #[error("transport error: {0}")]
    Transport(String),
}

pub type AppResult<T> = Result<T, AppError>;
