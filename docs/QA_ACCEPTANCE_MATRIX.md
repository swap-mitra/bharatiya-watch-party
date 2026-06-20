# QA Acceptance Matrix

This matrix is the Sprint 8 exit gate. Every row is a single testable
assertion. The matrix is **automated where possible** and **manual** where
the platform or media requires human verification.

## Status legend

- **Automated** — covered by a test in the regression suite, executed on
  every push.
- **Manual** — covered by a documented procedure; the result is recorded in
  the release notes.

## Functional acceptance

| ID    | Capability                                    | Verification                                                                                  | Status   |
| ----- | --------------------------------------------- | --------------------------------------------------------------------------------------------- | -------- |
| F-01  | Host can create a room                        | `signal_service::tests::create_room_assigns_unique_code`                                      | Automated |
| F-02  | Viewer can join a room with a valid room code  | `signal_service::tests::join_room_assigns_session_and_snapshot`                              | Automated |
| F-03  | Room full behavior is enforced                | `signal_service::tests::full_room_fanout` and `viewer_join_returns_conflict_when_full`        | Automated |
| F-04  | Chat works across connected participants      | `signal_service::tests::chat_round_trip_is_broadcast`                                         | Automated |
| F-05  | Host playback commands reach viewers          | `signal_service::tests::playback_command_fanout_to_viewers`                                   | Automated |
| F-06  | Viewer playback commands are rejected         | `signal_service::tests::viewer_playback_command_is_rejected`                                  | Automated |
| F-07  | Viewer playback heartbeats are rejected       | `signal_service::tests::viewer_playback_heartbeats_are_rejected`                              | Automated |
| F-08  | Reconnect uses the same session id            | `signal_service::tests::reconnect_with_same_session_id`                                       | Automated |
| F-09  | Host reconnect grace keeps the room open      | `signal_service::tests::host_disconnect_keeps_room_open_within_grace`                         | Automated |
| F-10  | Host grace expiry closes the room             | `signal_service::tests::host_disconnect_closes_room_after_grace_expires`                      | Automated |
| F-11  | Duplicate chat ids are suppressed             | `signal_service::tests::duplicate_chat_ids_are_suppressed`                                    | Automated |
| F-12  | Late-join chat replay is bounded              | `signal_service::tests::late_join_receives_bounded_chat_replay`                               | Automated |
| F-13  | Full-room fanout updates metrics              | `signal_service::tests::full_room_playback_fanout_updates_metrics`                            | Automated |

## Media acceptance

| ID    | Capability                                            | Verification                                                                                     | Status   |
| ----- | ----------------------------------------------------- | ------------------------------------------------------------------------------------------------ | -------- |
| M-01  | libmpv dynamic loader path candidates are documented  | `desktop_shell::tests::adapter_loads_stream_and_selects_tracks` (mock path) and `scripts/check-libmpv.ps1` (discovery dry-run) | Automated |
| M-02  | HLS playback via libmpv                               | Manual: load `https://test-streams.mux.dev/x36xhzz/x36xhzz.m3u8` on a host, observe viewers playing in sync. Record outcome in `docs/release/RELEASE_NOTES_<version>.md`. | Manual |
| M-03  | DASH playback via libmpv                              | Manual: load `https://test-streams.mux.dev/test_001/stream.mpd` on a host. Verify in `tauri dev` with libmpv installed. Record outcome. | Manual |
| M-04  | MP4 playback via libmpv or browser fallback           | Automated fallback: `apps/desktop` "Test a stream" loads Big Buck Bunny by default. Manual: verify DASH/HLS via libmpv. | Mixed |
| M-05  | Subtitle selection when available                     | Manual: load an HLS stream with multiple `sub` tracks, switch between them via the track selector. Record outcome. | Manual |
| M-06  | Alternate audio-track selection when available        | Manual: load an HLS stream with multiple `audio` tracks, switch via the audio selector. Record outcome. | Manual |

## Platform acceptance

| ID    | Capability                                              | Verification                                                                                     | Status   |
| ----- | ------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | -------- |
| P-01  | Windows local build (`cargo run -p desktop-shell`)      | Manual smoke test on a Windows host with WebView2 installed.                                  | Manual |
| P-02  | Windows packaged build (`npm run desktop:build`)        | `pwsh ./scripts/build-desktop.ps1 -Platform windows`; manual install of MSI or NSIS, then smoke test. | Manual |
| P-03  | macOS local build (`cargo tauri dev`)                   | Manual smoke test on a macOS host.                                                              | Manual |
| P-04  | macOS packaged build (`npm run desktop:build`)          | `bash ./scripts/build-desktop.sh -p macos`; manual install of APP/DMG, then smoke test.        | Manual |
| P-05  | Linux dev only                                          | CI runs `cargo fmt`, `cargo clippy`, `cargo test` on Ubuntu. Linux is not a release target.   | Automated |

## Network acceptance

| ID    | Capability                                                | Verification                                                                                     | Status   |
| ----- | --------------------------------------------------------- | ------------------------------------------------------------------------------------------------ | -------- |
| N-01  | Stable LAN / broadband scenario                           | Manual: host + 2 viewers on the same LAN; verify sync over 5 minutes.                            | Manual |
| N-02  | Moderate-latency network scenario                         | Manual: throttle the host's network to 250 ms RTT; verify drift correction converges.            | Manual |
| N-03  | Disconnect and reconnect scenario                         | Automated: `signal_service::tests::reconnect_with_same_session_id`. Manual: kill the host's network for 30 s and verify recovery. | Mixed |
| N-04  | Full-room load scenario                                   | Automated: `signal_service::tests::full_room_playback_fanout_updates_metrics`. Manual: 1 host + 10 viewers desktop load test. | Mixed |

## Regression suite

| ID    | Capability                                  | Verification                                                                                | Status   |
| ----- | ------------------------------------------- | ------------------------------------------------------------------------------------------- | -------- |
| R-01  | Rust unit and integration tests             | `cargo test --workspace`                                                                    | Automated |
| R-02  | Rust formatting                             | `cargo fmt --all -- --check`                                                                | Automated |
| R-03  | Rust linting                                | `cargo clippy --workspace --all-targets -- -D warnings`                                     | Automated |
| R-04  | Frontend typecheck                          | `npm run typecheck`                                                                         | Automated |
| R-05  | Frontend lint                               | `npm run lint`                                                                              | Automated |
| R-06  | Frontend production build                   | `npm run desktop:build`                                                                     | Automated |
| R-07  | Manual smoke test of create/join/watch/chat | Manual: host creates a room, a viewer joins, host plays a stream, both participants chat. | Manual |
| R-08  | Verification script runs the full suite     | `pwsh ./scripts/verify.ps1` exits `0`                                                       | Automated |
| R-09  | Tauri bundle config validates               | `npm run -w @watchparty/desktop tauri -- info` succeeds                                     | Automated |
| R-10  | libmpv discovery dry-run                    | `pwsh ./scripts/check-libmpv.ps1` prints a candidate or fails cleanly                        | Automated |

## Sprint 8 exit criteria

- [x] Specs through Sprint 8 are written (`docs/specs/00`–`18`).
- [x] Foundation flows are implemented (Sprints 1–7).
- [x] `verify.ps1` passes on the release commit.
- [x] `docs/release/RELEASE.md`, `SUPPORT_MATRIX.md`, `RELEASE_NOTES_TEMPLATE.md`,
      `SIGNAL_SERVICE_DEPLOYMENT.md`, `LIBMPV_BUNDLING.md`, and
      `CONFIGURATION.md` are written.
- [x] Tauri bundle config produces platform-specific installers for the
      supported OS list.
- [x] No signing certificates or secrets are required to build or publish a
      release.
- [ ] Manual certification matrix is signed off on a real Windows host.
- [ ] Manual certification matrix is signed off on a real macOS host.

The first six items are part of this sprint and are completed by the work in
this PR. The last two items are environment-dependent and recorded in
`docs/release/RELEASE_NOTES_<version>.md` per platform once the certification
run is performed.
