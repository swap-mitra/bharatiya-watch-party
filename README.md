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
- Frontend sync policy extracted into `apps/desktop/src/lib/playbackSync.ts` so heartbeat and drift behavior can evolve independently of the UI shell
- CI for Rust and frontend checks

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
- TURN/STUN, hosted fallback transport strategy, and deeper observability are not implemented yet
- Packaging and bundling of `libmpv` for distribution still needs to be finished for release builds

## Detailed Tracking

Spec-by-spec implementation status is tracked in `docs/implementation-status.md`.

## Tracking Rule

This README should be updated whenever implementation meaningfully changes so the repo has a current human-readable progress log alongside the specs and code.

## Spec Coverage

- `00` to `08`: foundation, protocol, backend, UI, observability, and tests
- `09` to `18`: room UX, `libmpv`, reconnects, sync correction, chat/presence, networking, observability implementation, performance, packaging, and QA/release acceptance




