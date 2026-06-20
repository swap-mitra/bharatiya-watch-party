//! Integration coverage for the Sprint 8 release-critical configuration
//! surface. The signal service is intentionally configurable via environment
//! variables; these tests pin the env contract so future refactors don't
//! silently break it.
//!
//! Env vars are global state, so every test acquires the `ENV_LOCK` mutex
//! before reading or writing them. This keeps parallel test execution from
//! poisoning the env surface.

use std::env;
use std::sync::Mutex;

use signal_service::{NetworkingConfig, ServiceConfig};

static ENV_LOCK: Mutex<()> = Mutex::new(());

const ENV_KEYS: &[&str] = &[
    "BIND_ADDR",
    "ROOM_TTL_SECONDS",
    "DISCONNECT_GRACE_SECONDS",
    "CORS_ALLOWED_ORIGINS",
    "STUN_URLS",
    "TURN_URLS",
    "TURN_USERNAME",
    "TURN_CREDENTIAL",
];

struct EnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl EnvGuard {
    fn acquire() -> Self {
        let lock = ENV_LOCK.lock().unwrap_or_else(|poison| poison.into_inner());
        for key in ENV_KEYS {
            unsafe { env::remove_var(key) };
        }
        Self { _lock: lock }
    }

    fn set(&self, key: &str, value: &str) {
        unsafe { env::set_var(key, value) };
    }
}

#[test]
fn from_env_uses_defaults_when_unset() {
    let _env = EnvGuard::acquire();
    let config = ServiceConfig::from_env();

    assert_eq!(config.bind_addr, "0.0.0.0:4000");
    assert_eq!(config.room_ttl.as_secs(), 4 * 60 * 60);
    assert_eq!(config.disconnect_grace.as_secs(), 60);
    assert!(
        config
            .cors_allowed_origins
            .iter()
            .any(|origin| origin == "http://localhost:1420")
    );
    assert!(config.networking.stun_urls.is_empty());
    assert!(config.networking.turn_urls.is_empty());
    assert!(config.networking.turn_username.is_none());
    assert!(!config.networking.turn_credential_present);
}

#[test]
fn from_env_overrides_bind_addr() {
    let env = EnvGuard::acquire();
    env.set("BIND_ADDR", "127.0.0.1:5000");
    let config = ServiceConfig::from_env();
    assert_eq!(config.bind_addr, "127.0.0.1:5000");
}

#[test]
fn from_env_overrides_room_ttl() {
    let env = EnvGuard::acquire();
    env.set("ROOM_TTL_SECONDS", "900");
    let config = ServiceConfig::from_env();
    assert_eq!(config.room_ttl.as_secs(), 900);
}

#[test]
fn from_env_overrides_disconnect_grace() {
    let env = EnvGuard::acquire();
    env.set("DISCONNECT_GRACE_SECONDS", "30");
    let config = ServiceConfig::from_env();
    assert_eq!(config.disconnect_grace.as_secs(), 30);
}

#[test]
fn from_env_parses_cors_origins_csv() {
    let env = EnvGuard::acquire();
    env.set(
        "CORS_ALLOWED_ORIGINS",
        "https://signal.example.com, https://app.example.com ,",
    );
    let config = ServiceConfig::from_env();
    assert_eq!(
        config.cors_allowed_origins,
        vec![
            "https://signal.example.com".to_string(),
            "https://app.example.com".to_string(),
        ]
    );
}

#[test]
fn from_env_parses_stun_and_turn_urls() {
    let env = EnvGuard::acquire();
    env.set(
        "STUN_URLS",
        "stun:stun.l.google.com:19302,stun:stun.example.com:3478",
    );
    env.set("TURN_URLS", "turn:turn.example.com:3478");
    env.set("TURN_USERNAME", "alice");
    env.set("TURN_CREDENTIAL", "secret-value");
    let config = ServiceConfig::from_env();
    assert_eq!(
        config.networking.stun_urls,
        vec![
            "stun:stun.l.google.com:19302".to_string(),
            "stun:stun.example.com:3478".to_string(),
        ]
    );
    assert_eq!(
        config.networking.turn_urls,
        vec!["turn:turn.example.com:3478".to_string()]
    );
    assert_eq!(config.networking.turn_username.as_deref(), Some("alice"));
    assert!(config.networking.turn_credential_present);
}

#[test]
fn from_env_ignores_invalid_numeric_overrides() {
    let env = EnvGuard::acquire();
    env.set("ROOM_TTL_SECONDS", "not-a-number");
    env.set("DISCONNECT_GRACE_SECONDS", "");
    env.set("BIND_ADDR", "");
    let config = ServiceConfig::from_env();
    assert_eq!(config.room_ttl.as_secs(), 4 * 60 * 60);
    assert_eq!(config.disconnect_grace.as_secs(), 60);
    assert_eq!(config.bind_addr, "0.0.0.0:4000");
}

#[test]
fn default_networking_config_is_disabled() {
    let config = NetworkingConfig::default();
    assert!(config.stun_urls.is_empty());
    assert!(config.turn_urls.is_empty());
    assert!(config.turn_username.is_none());
    assert!(!config.turn_credential_present);
}

#[test]
fn from_env_does_not_leak_turn_credential_value() {
    let env = EnvGuard::acquire();
    env.set("TURN_CREDENTIAL", "super-secret");
    let config = ServiceConfig::from_env();
    assert!(config.networking.turn_credential_present);
    let serialized = format!("{config:?}");
    assert!(!serialized.contains("super-secret"));
}
