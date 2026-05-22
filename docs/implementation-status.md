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
| `01-system-architecture-spec.md` | Partial | Tauri desktop app, Rust shared core, Rust signal service, WebSocket control plane, in-memory rooms, and native/browser player boundary exist. | Production deployment topology, hosted service config, observability, and packaging architecture are incomplete. |
| `02-player-spec.md` | Partial | `PlayerAdapter` exists; Tauri bridge supports load/play/pause/seek/stop/state/tracks; dynamic `libmpv` adapter exists; browser `<video>` fallback exists; local playback harness exists on the landing screen. | `libmpv` bundling is Sprint 8 release work; DASH support requires working native playback; manual media certification is pending. |
| `03-realtime-protocol-spec.md` | Implemented | Create/join HTTP flows, WebSocket messages, presence, chat, playback commands, room close, errors, and playback heartbeat schema exist. | Protocol versioning and compatibility policy are not implemented. |
| `04-sync-engine-spec.md` | Partial | Host-authoritative commands, host playback heartbeats, late-join playback snapshots, and first-pass viewer drift correction exist. | Playback-rate smoothing, monotonic-clock sync math, buffer-aware start, and measured threshold tuning are pending. |
| `05-room-service-spec.md` | Implemented | Room create/join, max 10 viewers, duplicate-name rejection, host-only playback mutations, presence, readiness, chat, close, expiry sweep, and in-memory state exist. | Persistent/storage-backed rooms are intentionally out of v1 scope. |
| `06-desktop-ui-spec.md` | Partial | Create/join screens, client-side form validation, room summary, copyable room code, player surface, host/viewer control states, standard/theater mode, side/bottom chat layout, presence, readiness meter, compact chat constraints, and room states exist. | UI needs accessibility review, responsive/manual testing, and final production polish. |
| `07-observability-and-performance-spec.md` | Not started | Basic event log exists in the desktop UI. | Structured logs, correlation IDs, metrics, drift telemetry, rebuffer tracking, and performance dashboards are pending. |
| `08-test-spec.md` | Partial | Rust unit/integration tests cover protocol, validation, room lifecycle, host authorization, reconnect, and heartbeat behavior; frontend typecheck/lint/build pass. | Desktop/player integration tests, UI tests, sync drift tests, load tests, and manual media certification are pending. |
| `09-room-experience-spec.md` | Partial | Host/viewer room flows, room code sharing, validated join flow, lobby, readiness, active room, authority messaging, chat, reconnecting, and closed-room surfaces exist. | Full UX QA across edge cases and multi-client manual testing are pending. |
| `10-libmpv-integration-spec.md` | Partial | Dynamic `libmpv` loading, native commands, state polling, track discovery, and track selection exist. | Cross-platform library bundling, installer integration, and media matrix certification are pending. |
| `11-session-and-reconnect-spec.md` | Partial | Viewer reconnect with same session id exists; reconnect UI state exists; room close handling exists. | Host reconnect grace period is not implemented; host disconnect currently closes the room. |
| `12-sync-correction-spec.md` | Partial | Host heartbeats and seek-based viewer correction thresholds exist in `apps/desktop/src/lib/playbackSync.ts`. | Monotonic clock sync, playback-rate correction, jitter smoothing, and measured threshold tuning are pending. |
| `13-chat-and-presence-spec.md` | Implemented | Text chat, presence list, readiness, connected/offline participant state, and 500-character chat validation exist. | Duplicate message suppression and richer delivery/reconnect behavior are pending. |
| `14-turn-stun-and-networking-spec.md` | Not started | Baseline control plane uses WebSocket; media still loads directly on each client. | TURN/STUN strategy, WebRTC signaling, and network fallback design are pending. |
| `15-observability-implementation-spec.md` | Not started | No production observability implementation yet. | Structured service logs, app diagnostics, sync metrics, and exportable debug bundles are pending. |
| `16-performance-and-load-spec.md` | Not started | Viewer cap is enforced at 10. | 10-viewer load test, latency measurement, sync drift measurement, CPU/memory profiling, and regression thresholds are pending. |
| `17-packaging-and-release-spec.md` | Not started | Tauri bundle config exists as a scaffold. | Windows/macOS signing, installers, `libmpv` bundling, release artifacts, and release docs are pending. |
| `18-qa-and-acceptance-spec.md` | Not started | Specs exist and baseline automated checks run. | End-to-end QA matrix, media acceptance, platform acceptance, network acceptance, and Sprint 8 exit certification are pending. |

## Sprint-Level Reality

| Sprint | Status | Notes |
| --- | --- | --- |
| Sprint 1 | Implemented | Repo scaffold, specs, shared core/domain/protocol, baseline checks. |
| Sprint 2 | Implemented | Signal service room lifecycle and host authority. |
| Sprint 3 | Implemented | Native player foundation, Tauri commands/events, dynamic `libmpv`, browser fallback, track controls, and local playback harness are in code. Release-grade `libmpv` bundling moves to Sprint 8. |
| Sprint 4 | Implemented | Watch-room UI, create/join validation, room code sharing, lobby readiness, standard/theater layouts, compact chat constraints, host/viewer authority states, and reconnect/closed surfaces are in code. Manual multi-client UX certification moves to Sprint 8 QA. |
| Sprint 5 | Partial | Heartbeat sync and seek drift correction exist; smoothing/tuning remain. |
| Sprint 6 | Partial | Chat/presence exist; reconnect behavior exists for viewers; duplicate suppression and deeper resilience remain. |
| Sprint 7 | Not started | TURN/networking, observability, and performance testing are not implemented. |
| Sprint 8 | Not started | Packaging, signing, release matrix, and QA certification are not implemented. |

## Highest-Priority Remaining Work

1. Continue Sprint 5 in order: tune playback sync behavior beyond first-pass heartbeat seek correction.
2. Add real observability: room/session IDs, sync drift logs, playback errors, reconnect events, and backend request logs.
3. Run 2-client and 10-viewer sync/load tests and tune heartbeat correction thresholds.
4. Add host reconnect grace period instead of closing the room immediately on host disconnect.
5. Package and bundle `libmpv` for Windows and macOS during Sprint 8 release work.
6. Execute the QA acceptance matrix from `18-qa-and-acceptance-spec.md`.
