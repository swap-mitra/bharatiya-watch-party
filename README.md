# Bharatiya Watch Party

Cross-platform desktop watch party app for macOS and Windows with a Rust core, WebSocket signaling, and a desktop UI shell built with Tauri, React, and TypeScript.

## Workspace

- `apps/desktop`: desktop shell, room flows, and Tauri frontend
- `crates/app-core`: shared domain types, validation, protocol, and player contract
- `services/signal-service`: Rust signaling service with ephemeral room state
- `docs/specs`: source-of-truth product, protocol, backend, UI, and test specs

## Current Status

### Implemented

- Rust workspace scaffold with shared domain crate, signaling service, and Tauri desktop shell
- Source-controlled specs in `docs/specs/00` through `docs/specs/18`
- Room creation and join over HTTP
- WebSocket room attachment, presence updates, readiness updates, chat, host-authoritative playback commands, and explicit host room closure
- Desktop watch-party shell with:
  - create room flow
  - join room flow
  - client-side create/join validation
  - copyable room code in active rooms
  - room summary and participant list
  - lobby readiness meter
  - standard and theater layout modes
  - room chat panel
  - compact chat character limits
  - host/viewer playback authority messaging
  - Tauri player command/event bridge
- Restrained neo-brutalist UI redesign with:
  - player-first room composition
  - chat as the primary room rail
  - theater toggle colocated with the player
  - reduced diagnostics and nonessential copy in the main experience
  - muted dark palette, hard borders, and minimal accent color
  - reduced room chrome and compact control dock so the player remains the dominant surface
- UI resilience for room entry:
  - welcome payloads tolerate missing chat history from stale/local signal-service builds
  - React error boundary shows a recoverable app-level fault screen instead of the default crash page
- Sprint 3 native playback foundation with:
  - `PlayerAdapter` contract in the shared Rust domain crate
  - dynamic `libmpv` loading in the Tauri backend
  - direct media loading, play, pause, seek, stop, and state polling
  - audio track and subtitle track discovery and selection
  - frontend bootstrap reporting for native vs browser fallback backend mode
  - browser video fallback when `libmpv` is not installed yet or the frontend runs in plain web mode
  - local playback harness on the landing screen for player testing without joining a room
- Reconnect-aware desktop room experience with dedicated lobby, reconnecting, and closed-room surfaces
- Host-authoritative playback heartbeats with conservative viewer drift correction and late-join timeline snapshots
- Playback-rate smoothing for medium drift before falling back to seek correction
- Duplicate-suppressed room chat with bounded recent-history replay for reconnecting and late-joining clients
- Signaling service observability endpoints:
  - `GET /metrics` for in-memory room, transport, chat, playback, validation, and fanout counters
  - `GET /networking` for the active WebSocket/direct-media network posture
- Structured service logs for room create/join/connect/disconnect/close, accepted chat, accepted playback commands, and playback fanout
- Full-room backend fanout test covering one host plus ten viewers
- Frontend sync policy extracted into `apps/desktop/src/lib/playbackSync.ts` so heartbeat and drift behavior can evolve independently of the UI shell
- CI for Rust and frontend checks
- GitHub Actions workflow runs on every push, pull request, and manual dispatch with Rust format/clippy/test plus frontend install/typecheck/lint/build

### Verified

- `cargo fmt --all`
- `cargo test --workspace`
- `npm run lint`
- `npm run typecheck`
- `npm run build --workspace @watchparty/desktop`

## Sprint Tracking

- Sprint 1: complete
- Sprint 2: complete
- Sprint 3: implemented end to end in code
  - Native playback uses `libmpv` when the shared library is available.
  - If `libmpv` is missing, the desktop app falls back to a real browser `<video>` playback surface and surfaces a warning in the UI.
  - The landing screen includes a local playback harness for direct URL testing before room sync.
- Sprint 4: implemented in code
  - Watch-room UI, create/join validation, room code sharing, readiness, compact chat, and host/viewer control states are built.
- Sprint 5: implemented in code
  - Host heartbeats, late-join sync, drift correction, playback-rate smoothing, and speed reset behavior are built.
- Sprint 6: implemented in code
  - Text chat, presence, viewer reconnect, duplicate message suppression, and bounded chat replay are built.
- Sprint 7: implemented in code
  - Service metrics, networking posture endpoint, lifecycle/fanout logs, and 10-viewer backend fanout verification are built.

## Native Playback Notes

- Primary playback target: `libmpv`
- Secondary media utility: `FFmpeg`

`FFmpeg` is not the primary player runtime. It remains useful for probing, diagnostics, and future media tooling.

The Tauri desktop backend will try to load `libmpv` from:
- `MPV_LIBRARY_PATH` if set
- Windows defaults such as `mpv-2.dll`, `libmpv-2.dll`, `mpv-1.dll`
- macOS defaults such as `libmpv.2.dylib`, `libmpv.dylib`

For full native playback on a collaborator machine, `libmpv` must be installed or bundled so the desktop app can load it at runtime.

If `libmpv` is missing on Windows, the desktop app will show a warning like `LoadLibraryExW failed` and fall back to browser video playback. That does not block room creation or signaling.

The browser fallback is intended for local development and smoke testing. It supports MP4/WebM and browser-native HLS where the embedded WebView supports it. DASH `.mpd` streams still require native `libmpv` playback or a future MSE/DASH integration.

## Local Development Notes

- Start the signaling service with `cargo run -p signal-service` before creating or joining rooms.
- The signal service allows local desktop/webview origins for development on `localhost:1420`, `127.0.0.1:1420`, and Tauri local origins.
- If room actions fail with `Could not reach the signal service`, verify that port `4000` is free and the backend is listening on `http://127.0.0.1:4000`.

## Current Gaps

- Playback synchronization now has host heartbeats, playback-rate smoothing, and seek-based drift correction; measured threshold tuning is still pending
- TURN/STUN peer transport remains disabled by design in v1; WebSocket signaling and direct client media fetch are the active model
- Production metrics export, hosted dashboards, frontend telemetry pipeline, and manual multi-client performance certification are still pending
- Packaging and bundling of `libmpv` for distribution still needs to be finished for release builds

## Detailed Tracking

Spec-by-spec implementation status is tracked in `docs/implementation-status.md`.

## Tracking Rule

This README should be updated whenever implementation meaningfully changes so the repo has a current human-readable progress log alongside the specs and code.

## Spec Coverage

- `00` to `08`: foundation, protocol, backend, UI, observability, and tests
- `09` to `18`: room UX, `libmpv`, reconnects, sync correction, chat/presence, networking, observability implementation, performance, packaging, and QA/release acceptance




