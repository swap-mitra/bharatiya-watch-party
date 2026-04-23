use rand::prelude::IndexedRandom;
use serde::{Deserialize, Serialize};
use std::fmt;
use uuid::Uuid;

use crate::{AppError, AppResult};

pub const ROOM_CODE_LENGTH: usize = 6;
pub const MAX_VIEWERS: usize = 10;
const ROOM_ALPHABET: &[u8] = b"ABCDEFGHJKLMNPQRSTUVWXYZ23456789";

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SessionId(pub Uuid);

impl SessionId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }
}

impl Default for SessionId {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for SessionId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct RoomCode(pub String);

impl RoomCode {
    pub fn generate() -> Self {
        let mut rng = rand::rng();
        let code = (0..ROOM_CODE_LENGTH)
            .map(|_| {
                *ROOM_ALPHABET
                    .choose(&mut rng)
                    .expect("alphabet is non-empty") as char
            })
            .collect::<String>();
        Self(code)
    }

    pub fn parse(value: impl AsRef<str>) -> AppResult<Self> {
        let normalized = value.as_ref().trim().to_uppercase();
        let is_valid = normalized.len() == ROOM_CODE_LENGTH
            && normalized
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit());

        if !is_valid {
            return Err(AppError::Validation(
                "room code must be 6 uppercase characters".into(),
            ));
        }

        Ok(Self(normalized))
    }
}

impl fmt::Display for RoomCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParticipantRole {
    Host,
    Viewer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Participant {
    pub session_id: SessionId,
    pub display_name: String,
    pub role: ParticipantRole,
    pub connected: bool,
    pub ready: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn generated_room_code_uses_expected_charset() {
        let code = RoomCode::generate();
        assert_eq!(code.0.len(), ROOM_CODE_LENGTH);
        assert!(
            code.0
                .chars()
                .all(|ch| ch.is_ascii_uppercase() || ch.is_ascii_digit())
        );
    }

    #[test]
    fn parses_and_normalizes_room_code() {
        let code = RoomCode::parse("ab12cd").expect("should parse");
        assert_eq!(code.0, "AB12CD");
    }
}
