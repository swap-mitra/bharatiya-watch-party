use app_core::{
    ClientMessage, CreateRoomRequest, JoinRoomRequest, PlaybackAction, PlaybackCommand,
    RoomCloseReason, ServerMessage,
};
use signal_service::{RoomRegistry, ServiceConfig};
use tokio::{
    sync::mpsc,
    time::{Duration, timeout},
};

#[tokio::test]
async fn room_full_is_rejected_after_ten_viewers() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");

    for index in 0..10 {
        registry
            .reserve_viewer(
                created.room_code.clone(),
                JoinRoomRequest {
                    display_name: format!("Viewer {}", index),
                },
            )
            .expect("viewer should be reserved");
    }

    let err = registry
        .reserve_viewer(
            created.room_code,
            JoinRoomRequest {
                display_name: "Viewer 11".into(),
            },
        )
        .expect_err("should reject the eleventh viewer");

    assert!(matches!(err, app_core::AppError::RoomFull));
}

#[tokio::test]
async fn viewer_playback_commands_are_rejected() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");
    let joined = registry
        .reserve_viewer(
            created.room_code.clone(),
            JoinRoomRequest {
                display_name: "Viewer".into(),
            },
        )
        .expect("viewer should join");

    let (host_tx, _host_rx) = mpsc::unbounded_channel();
    let (viewer_tx, mut viewer_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");
    registry
        .connect(created.room_code.clone(), joined.session_id, viewer_tx)
        .expect("viewer should connect");

    let result = registry.handle_client_message(
        &created.room_code,
        &joined.session_id,
        ClientMessage::PlaybackCommand(PlaybackCommand {
            seq: 1,
            action: PlaybackAction::Play,
            position_ms: None,
            stream_url: None,
            issued_at_ms: 10,
        }),
    );

    assert!(matches!(result, Err(app_core::AppError::Unauthorized)));
    registry.send_to(
        &created.room_code,
        &joined.session_id,
        ServerMessage::Error {
            code: "message_rejected".into(),
            message: "unauthorized action".into(),
        },
    );

    let received = recv_until(&mut viewer_rx, |message| {
        matches!(message, ServerMessage::Error { .. })
    })
    .await
    .expect("viewer should receive error");
    assert!(matches!(received, ServerMessage::Error { .. }));
}

#[tokio::test]
async fn host_disconnect_closes_the_room() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");
    let joined = registry
        .reserve_viewer(
            created.room_code.clone(),
            JoinRoomRequest {
                display_name: "Viewer".into(),
            },
        )
        .expect("viewer should join");

    let (host_tx, _host_rx) = mpsc::unbounded_channel();
    let (viewer_tx, mut viewer_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");
    registry
        .connect(created.room_code.clone(), joined.session_id, viewer_tx)
        .expect("viewer should connect");

    registry.disconnect(&created.room_code, &created.session_id);

    let closed = recv_until(&mut viewer_rx, |message| {
        matches!(
            message,
            ServerMessage::RoomClosed {
                reason: RoomCloseReason::HostDisconnected
            }
        )
    })
    .await
    .expect("viewer should receive room closed");
    assert_eq!(
        closed,
        ServerMessage::RoomClosed {
            reason: RoomCloseReason::HostDisconnected,
        }
    );
}

#[tokio::test]
async fn reconnect_uses_the_same_session_id() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");

    let (host_tx, _host_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");

    let viewer = registry
        .reserve_viewer(
            created.room_code.clone(),
            JoinRoomRequest {
                display_name: "Viewer".into(),
            },
        )
        .expect("viewer should join");

    let (first_tx, _first_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), viewer.session_id, first_tx)
        .expect("viewer should connect");
    registry.disconnect(&created.room_code, &viewer.session_id);

    let (second_tx, _second_rx) = mpsc::unbounded_channel();
    let welcome = registry
        .connect(created.room_code.clone(), viewer.session_id, second_tx)
        .expect("viewer should reconnect");

    match welcome {
        ServerMessage::Welcome {
            self_session_id, ..
        } => assert_eq!(self_session_id, viewer.session_id),
        _ => panic!("expected welcome message"),
    }
}

async fn recv_until(
    receiver: &mut mpsc::UnboundedReceiver<ServerMessage>,
    predicate: impl Fn(&ServerMessage) -> bool,
) -> Option<ServerMessage> {
    timeout(Duration::from_secs(1), async {
        while let Some(message) = receiver.recv().await {
            if predicate(&message) {
                return Some(message);
            }
        }
        None
    })
    .await
    .ok()
    .flatten()
}
