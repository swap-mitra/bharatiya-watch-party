use app_core::{
    ClientMessage, CreateRoomRequest, JoinRoomRequest, PlaybackAction, PlaybackCommand,
    PlaybackHeartbeat, PlayerStatus, RoomCloseReason, ServerMessage,
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
async fn viewer_playback_heartbeats_are_rejected() {
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
    let (viewer_tx, _viewer_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");
    registry
        .connect(created.room_code.clone(), joined.session_id, viewer_tx)
        .expect("viewer should connect");

    let result = registry.handle_client_message(
        &created.room_code,
        &joined.session_id,
        ClientMessage::PlaybackHeartbeat(PlaybackHeartbeat {
            command_seq: 1,
            position_ms: 42_000,
            status: PlayerStatus::Playing,
            active_source: Some("https://example.com/movie.mp4".into()),
            sent_at_ms: 10,
        }),
    );

    assert!(matches!(result, Err(app_core::AppError::Unauthorized)));
}

#[tokio::test]
async fn host_heartbeat_broadcasts_and_updates_late_join_snapshot() {
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

    registry
        .handle_client_message(
            &created.room_code,
            &created.session_id,
            ClientMessage::PlaybackCommand(PlaybackCommand {
                seq: 1,
                action: PlaybackAction::LoadStream,
                position_ms: Some(0),
                stream_url: Some("https://example.com/movie.mp4".into()),
                issued_at_ms: 10,
            }),
        )
        .expect("host command should be accepted");

    registry
        .handle_client_message(
            &created.room_code,
            &created.session_id,
            ClientMessage::PlaybackHeartbeat(PlaybackHeartbeat {
                command_seq: 1,
                position_ms: 42_000,
                status: PlayerStatus::Playing,
                active_source: Some("https://example.com/movie.mp4".into()),
                sent_at_ms: 20,
            }),
        )
        .expect("host heartbeat should be accepted");

    let heartbeat = recv_until(&mut viewer_rx, |message| {
        matches!(message, ServerMessage::PlaybackHeartbeat(_))
    })
    .await
    .expect("viewer should receive heartbeat");
    assert!(matches!(
        heartbeat,
        ServerMessage::PlaybackHeartbeat(PlaybackHeartbeat {
            command_seq: 1,
            position_ms: 42_000,
            status: PlayerStatus::Playing,
            ..
        })
    ));

    let late_viewer = registry
        .reserve_viewer(
            created.room_code.clone(),
            JoinRoomRequest {
                display_name: "Late Viewer".into(),
            },
        )
        .expect("late viewer should join");
    let (late_tx, _late_rx) = mpsc::unbounded_channel();
    let welcome = registry
        .connect(created.room_code.clone(), late_viewer.session_id, late_tx)
        .expect("late viewer should connect");

    match welcome {
        ServerMessage::Welcome { playback, .. } => {
            assert_eq!(playback.position_ms, 42_000);
            assert_eq!(playback.status, PlayerStatus::Playing);
            assert_eq!(
                playback.active_source,
                Some("https://example.com/movie.mp4".into())
            );
        }
        _ => panic!("expected welcome message"),
    }
}

#[tokio::test]
async fn host_heartbeat_cannot_replace_load_stream_command() {
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

    let result = registry.handle_client_message(
        &created.room_code,
        &created.session_id,
        ClientMessage::PlaybackHeartbeat(PlaybackHeartbeat {
            command_seq: 1,
            position_ms: 42_000,
            status: PlayerStatus::Playing,
            active_source: Some("https://example.com/movie.mp4".into()),
            sent_at_ms: 20,
        }),
    );

    assert!(matches!(result, Err(app_core::AppError::Validation(_))));
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
async fn host_can_close_room_explicitly() {
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

    let (host_tx, mut host_rx) = mpsc::unbounded_channel();
    let (viewer_tx, mut viewer_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");
    registry
        .connect(created.room_code.clone(), joined.session_id, viewer_tx)
        .expect("viewer should connect");

    registry
        .handle_client_message(
            &created.room_code,
            &created.session_id,
            ClientMessage::CloseRoom,
        )
        .expect("host should be able to close the room");

    let host_closed = recv_until(&mut host_rx, |message| {
        matches!(
            message,
            ServerMessage::RoomClosed {
                reason: RoomCloseReason::ClosedByHost
            }
        )
    })
    .await
    .expect("host should receive room closed");
    let viewer_closed = recv_until(&mut viewer_rx, |message| {
        matches!(
            message,
            ServerMessage::RoomClosed {
                reason: RoomCloseReason::ClosedByHost
            }
        )
    })
    .await
    .expect("viewer should receive room closed");

    assert_eq!(
        host_closed,
        ServerMessage::RoomClosed {
            reason: RoomCloseReason::ClosedByHost,
        }
    );
    assert_eq!(
        viewer_closed,
        ServerMessage::RoomClosed {
            reason: RoomCloseReason::ClosedByHost,
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

#[tokio::test]
async fn duplicate_chat_ids_are_suppressed() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");

    let (host_tx, mut host_rx) = mpsc::unbounded_channel();
    registry
        .connect(created.room_code.clone(), created.session_id, host_tx)
        .expect("host should connect");

    for _ in 0..2 {
        registry
            .handle_client_message(
                &created.room_code,
                &created.session_id,
                ClientMessage::ChatSend {
                    id: "chat-1".into(),
                    text: "Hello room".into(),
                },
            )
            .expect("chat should be accepted");
    }

    let first_chat = recv_until(
        &mut host_rx,
        |message| matches!(message, ServerMessage::Chat(chat) if chat.id == "chat-1"),
    )
    .await
    .expect("first chat should broadcast");
    assert!(matches!(first_chat, ServerMessage::Chat(_)));

    let duplicate = recv_until(
        &mut host_rx,
        |message| matches!(message, ServerMessage::Chat(chat) if chat.id == "chat-1"),
    )
    .await;
    assert!(
        duplicate.is_none(),
        "duplicate chat id should not rebroadcast"
    );

    let metrics = registry.metrics_snapshot();
    assert_eq!(metrics.chat_message_count, 1);
}

#[tokio::test]
async fn reconnect_welcome_includes_recent_chat_history() {
    let registry = RoomRegistry::new(ServiceConfig::default());
    let created = registry
        .create_room(CreateRoomRequest {
            display_name: "Host".into(),
        })
        .expect("room should be created");
    let viewer = registry
        .reserve_viewer(
            created.room_code.clone(),
            JoinRoomRequest {
                display_name: "Viewer".into(),
            },
        )
        .expect("viewer should join");

    registry
        .handle_client_message(
            &created.room_code,
            &created.session_id,
            ClientMessage::ChatSend {
                id: "chat-history-1".into(),
                text: "Bring popcorn".into(),
            },
        )
        .expect("chat should be accepted");

    let (viewer_tx, _viewer_rx) = mpsc::unbounded_channel();
    let welcome = registry
        .connect(created.room_code.clone(), viewer.session_id, viewer_tx)
        .expect("viewer should connect");

    match welcome {
        ServerMessage::Welcome { chat_history, .. } => {
            assert_eq!(chat_history.len(), 1);
            assert_eq!(chat_history[0].id, "chat-history-1");
            assert_eq!(chat_history[0].text, "Bring popcorn");
        }
        _ => panic!("expected welcome message"),
    }
}

#[tokio::test]
async fn full_room_playback_fanout_updates_metrics() {
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

    let mut viewer_receivers = Vec::new();
    for index in 0..10 {
        let viewer = registry
            .reserve_viewer(
                created.room_code.clone(),
                JoinRoomRequest {
                    display_name: format!("Viewer {}", index),
                },
            )
            .expect("viewer should be reserved");
        let (viewer_tx, viewer_rx) = mpsc::unbounded_channel();
        registry
            .connect(created.room_code.clone(), viewer.session_id, viewer_tx)
            .expect("viewer should connect");
        viewer_receivers.push(viewer_rx);
    }

    registry
        .handle_client_message(
            &created.room_code,
            &created.session_id,
            ClientMessage::PlaybackCommand(PlaybackCommand {
                seq: 1,
                action: PlaybackAction::Play,
                position_ms: None,
                stream_url: None,
                issued_at_ms: 10,
            }),
        )
        .expect("host command should be accepted");

    for viewer_rx in &mut viewer_receivers {
        let playback = recv_until(
            viewer_rx,
            |message| matches!(message, ServerMessage::Playback(command) if command.seq == 1),
        )
        .await
        .expect("viewer should receive playback command");
        assert!(matches!(playback, ServerMessage::Playback(_)));
    }

    let metrics = registry.metrics_snapshot();
    assert_eq!(metrics.room_create_count, 1);
    assert_eq!(metrics.room_join_count, 10);
    assert_eq!(metrics.active_room_count, 1);
    assert_eq!(metrics.active_participant_count, 11);
    assert_eq!(metrics.playback_command_count, 1);
    assert_eq!(metrics.playback_fanout_count, 1);
    assert!(metrics.outbound_message_count >= 11);
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
