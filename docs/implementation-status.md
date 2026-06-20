# Implementation Status

This document tracks implementation against the source specs in `docs/specs`.

Status key:

- `Implemented`: covered in code and has at least baseline verification
- `Partial`: meaningful code exists, but acceptance criteria are not fully met
- `Not started`: spec exists, but implementation has not begun
- `Blocked / pending manual`: implementation depends on packaging, environment setup, or manual certification

## Spec Coverage Matrix

| Spec | Status | Implemented So Far | Remaining Work |
| --- | --- | --- | --- |
| `00-product-spec.md` | Partial | Desktop-only watch party MVP direction is reflected in repo structure, room flow, host control, chat, and direct media URL contract. | Product is not release-ready; still needs packaging, media certification, load testing, and QA gates. |
| `01-system-architecture-spec.md` | Partial | Tauri desktop app, Rust shared core, Rust signal service, WebSocket control plane, in-memory rooms, native/browser player boundary, service metrics, and explicit networking posture endpoint exist. | Production deployment topology, hosted service config, production metrics export, and packaging architecture are incomplete. |
| `02-player-spec.md` | Partial | `PlayerAdapter` exists; Tauri bridge supports load/play/pause/seek/stop/state/tracks; dynamic `libmpv` adapter exists; browser `<video>` fallback exists; local playback harness exists on the landing screen. | `libmpv` bundling is Sprint 8 release work; DASH support requires working native playback; manual media certification is pending. |
| `03-realtime-protocol-spec.md` | Implemented | Create/join HTTP flows, WebSocket messages, presence, duplicate-suppressed chat, chat replay in welcome payloads, playback commands, room close, errors, and playback heartbeat schema exist. | Protocol versioning and compatibility policy are not implemented. |
| `04-sync-engine-spec.md` | Partial | Host-authoritative commands, host playback heartbeats, late-join playback snapshots, seek correction, and playback-rate smoothing exist. | Buffer-aware start, monotonic-clock refinement, and measured threshold tuning are pending. |
| `05-room-service-spec.md` | Implemented | Room create/join, max 10 viewers, duplicate-name rejection, host-only playback mutations, presence, readiness, chat, close, expiry sweep, and in-memory state exist. | Persistent/storage-backed rooms are intentionally out of v1 scope. |
| `06-desktop-ui-spec.md` | Partial | Create/join screens, client-side form validation, compact room summary, copyable room code, player-first neo-brutalist room surface, compact playback control dock, theater toggle colocated with the player, standard/theater mode, side/bottom chat layout, presence, readiness meter, compact chat constraints, room states, stale welcome-payload tolerance, and an app error boundary exist. | UI needs visual QA, accessibility review, responsive/manual testing, and final production polish. |
| `07-observability-and-performance-spec.md` | Partial | Service metrics snapshot and `GET /metrics` exist; structured lifecycle/fanout logs exist; active participant count and playback fanout timing are tracked; full-room backend fanout is tested. | Production metrics sink, correlation IDs, drift telemetry export, rebuffer tracking, frontend telemetry pipeline, and dashboards are pending. |
| `08-test-spec.md` | Partial | Rust unit/integration tests cover protocol, validation, room lifecycle, host authorization, reconnect, heartbeat behavior, duplicate chat suppression, chat replay, metrics, and 10-viewer playback fanout; frontend typecheck/lint/build pass. | Desktop/player integration tests, UI tests, sync drift tests, broader load tests, and manual media certification are pending. |
| `09-room-experience-spec.md` | Partial | Host/viewer room flows, room code sharing, validated join flow, lobby, readiness, active room, authority messaging, chat, reconnecting, and closed-room surfaces exist. | Full UX QA across edge cases and multi-client manual testing are pending. |
| `10-libmpv-integration-spec.md` | Partial | Dynamic `libmpv` loading, native commands, state polling, track discovery, and track selection exist. | Cross-platform library bundling, installer integration, and media matrix certification are pending. |
| `11-session-and-reconnect-spec.md` | Implemented | Viewer reconnect with same session id, host reconnect grace using `disconnect_grace`, room close on host grace expiry, presence reflects host disconnect/reconnect within grace, reconnect welcome payloads replay recent chat history, host authority restored on reconnect all exist with test coverage. | Persistent multi-session resume and cross-device session migration are intentionally out of v1 scope. |
| `12-sync-correction-spec.md` | Partial | Host heartbeats, seek correction, playback-rate smoothing, jitter bounds, and correction throttling exist in `apps/desktop/src/lib/playbackSync.ts`. | Measured threshold tuning and multi-client certification are pending. |
| `13-chat-and-presence-spec.md` | Implemented | Text chat, presence list, readiness, connected/offline participant state, 500-character chat validation, client message IDs, server duplicate suppression, frontend duplicate merge, and bounded chat replay exist. | Persistent chat history, delivery receipts, and moderation are intentionally out of v1 scope. |
| `14-turn-stun-and-networking-spec.md` | Partial | Baseline control plane uses WebSocket; media loads directly on each client; `GET /networking` exposes WebSocket/direct-media as active and WebRTC/STUN/TURN as disabled. | Future WebRTC signaling and configurable STUN/TURN relay behavior are pending and remain out of the active v1 path. |
| `15-observability-implementation-spec.md` | Partial | Structured service logs cover room create, join, connect, reconnect, disconnect, close, chat, playback command, HTTP create/join latency, and playback fanout; `GET /metrics` exposes room, transport, chat, playback, validation, outbound, and fanout counters. | Production sink, distributed tracing, frontend telemetry, sync drift export, player failure categorization, and debug bundles are pending. |
| `16-performance-and-load-spec.md` | Partial | Integration test verifies one host plus ten viewers receive playback fanout; metrics assert active participants, joins, playback commands, fanout, and outbound message counts. | Manual 10-viewer desktop load testing, latency measurements, CPU/memory profiling, burst regression thresholds, and recorded release values are pending. |
| `17-packaging-and-release-spec.md` | Not started | Tauri bundle config exists as a scaffold. | Windows/macOS signing, installers, `libmpv` bundling, release artifacts, and release docs are pending. |
| `18-qa-and-acceptance-spec.md` | Not started | Specs exist and baseline automated checks run. | End-to-end QA matrix, media acceptance, platform acceptance, network acceptance, and Sprint 8 exit certification are pending. |

## Sprint-Level Reality

| Sprint | Status | Notes |
| --- | --- | --- |
| Sprint 1 | Implemented | Repo scaffold, specs, shared core/domain/protocol, baseline checks. |
| Sprint 2 | Implemented | Signal service room lifecycle and host authority. |
| Sprint 3 | Implemented | Native player foundation, Tauri commands/events, dynamic `libmpv`, browser fallback, track controls, and local playback harness are in code. Release-grade `libmpv` bundling moves to Sprint 8. |
| Sprint 4 | Implemented | Watch-room UI, create/join validation, room code sharing, lobby readiness, standard/theater layouts, compact chat constraints, host/viewer authority states, and reconnect/closed surfaces are in code. Manual multi-client UX certification moves to Sprint 8 QA. |
| Sprint 5 | Implemented | Host heartbeats, late-join playback snapshots, seek correction, playback-rate smoothing, correction throttling, and sync speed reset are in code. Multi-client tuning moves to Sprint 8 QA/performance. |
| Sprint 6 | Implemented | Text chat, presence, readiness, viewer reconnect, duplicate chat suppression, bounded reconnect/late-join chat replay, and host reconnect grace with sweep-based room close are in code. |
| Sprint 7 | Implemented | Service metrics, networking posture endpoint, structured lifecycle/fanout logs, and one-host-plus-ten-viewer backend fanout verification are in code. Production dashboards and manual certification move to Sprint 8 QA/release work. |
| Sprint 8 | Not started | Packaging, signing, release matrix, and QA certification are not implemented. |

## Highest-Priority Remaining Work

1. Continue Sprint 8 in order: packaging, signing, release matrix, and QA certification.
2. Run 2-client and 10-viewer desktop sync/load tests and tune heartbeat correction thresholds.
3. Package and bundle `libmpv` for Windows and macOS during Sprint 8 release work.
4. Add production metrics export/dashboard wiring if a hosting target is chosen.
5. Execute the QA acceptance matrix from `18-qa-and-acceptance-spec.md`.
