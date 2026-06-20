//! HTTP-level smoke tests for the Sprint 8 release-critical networking and
//! CORS endpoints. These tests start an in-memory router with a known
//! `ServiceConfig` and assert that the JSON output matches the config.
//!
//! The tests run in-process and never touch the network.

use std::env;
use std::sync::Mutex;

use axum::body::Body;
use axum::http::{Request, StatusCode};
use signal_service::{AppState, NetworkingConfig, ServiceConfig, app_router_with_config};
use tower::ServiceExt;

static ROUTER_LOCK: Mutex<()> = Mutex::new(());

fn reset_env() {
    for key in [
        "BIND_ADDR",
        "ROOM_TTL_SECONDS",
        "DISCONNECT_GRACE_SECONDS",
        "CORS_ALLOWED_ORIGINS",
        "STUN_URLS",
        "TURN_URLS",
        "TURN_USERNAME",
        "TURN_CREDENTIAL",
    ] {
        unsafe { env::remove_var(key) };
    }
}

fn reset_env_locked() {
    let _guard = ROUTER_LOCK
        .lock()
        .unwrap_or_else(|poison| poison.into_inner());
    reset_env();
}

async fn fetch_json(router: axum::Router, path: &str) -> (StatusCode, serde_json::Value) {
    let response = router
        .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
        .await
        .expect("router should respond");
    let status = response.status();
    let body = axum::body::to_bytes(response.into_body(), 1024)
        .await
        .unwrap();
    let value: serde_json::Value = serde_json::from_slice(&body).unwrap_or(serde_json::Value::Null);
    (status, value)
}

#[tokio::test]
async fn networking_endpoint_reports_disabled_state_by_default() {
    reset_env_locked();
    let router = app_router_with_config(
        AppState::new(ServiceConfig::default()),
        ServiceConfig::default(),
    );
    let (status, body) = fetch_json(router, "/networking").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["signalingTransport"], "websocket");
    assert_eq!(body["mediaTransport"], "direct-client-fetch");
    assert_eq!(body["webrtcEnabled"], false);
    assert_eq!(body["stunConfigured"], false);
    assert_eq!(body["turnConfigured"], false);
    assert_eq!(body["fallbackTransport"], "hosted-websocket-signaling");
}

#[tokio::test]
async fn networking_endpoint_reports_webrtc_enabled_when_stun_is_set() {
    reset_env_locked();
    let config = ServiceConfig {
        networking: NetworkingConfig {
            stun_urls: vec!["stun:stun.example.com:3478".into()],
            ..NetworkingConfig::default()
        },
        ..ServiceConfig::default()
    };
    let router = app_router_with_config(AppState::new(config.clone()), config);
    let (status, body) = fetch_json(router, "/networking").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["stunConfigured"], true);
    assert_eq!(body["webrtcEnabled"], true);
    assert_eq!(body["turnConfigured"], false);
}

#[tokio::test]
async fn networking_endpoint_reports_turn_when_credential_present() {
    reset_env_locked();
    let config = ServiceConfig {
        networking: NetworkingConfig {
            turn_credential_present: true,
            ..NetworkingConfig::default()
        },
        ..ServiceConfig::default()
    };
    let router = app_router_with_config(AppState::new(config.clone()), config);
    let (status, body) = fetch_json(router, "/networking").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["turnConfigured"], true);
}

#[tokio::test]
async fn health_endpoint_reports_ok() {
    reset_env_locked();
    let router = app_router_with_config(
        AppState::new(ServiceConfig::default()),
        ServiceConfig::default(),
    );
    let (status, body) = fetch_json(router, "/health").await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["ok"], true);
}
