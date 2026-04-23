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
- Source-controlled specs in `docs/specs/00` through `docs/specs/08`
- Room creation and join over HTTP
- WebSocket room attachment, presence updates, readiness updates, chat, and host-authoritative playback commands
- Desktop watch-party shell with:
  - create room flow
  - join room flow
  - room summary and participant list
  - standard and theater layout modes
  - room chat panel
  - Tauri player command/event bridge
- CI for Rust and frontend checks

### Verified

- `cargo fmt --all`
- `cargo test --workspace`
- `npm run lint`
- `npm run typecheck`
- `npm run build --workspace @watchparty/desktop`

## Current Gaps

- The desktop player still uses a harness/stub adapter; real `libmpv` integration is not implemented yet
- Playback synchronization is command replication only; drift correction and late-join sync refinement are still pending
- TURN/STUN, hosted fallback transport strategy, and deeper observability are not implemented yet

## Media Stack Decision

- Primary playback target: `libmpv`
- Secondary media utility: `FFmpeg`

`FFmpeg` is not the primary player runtime. It remains useful for probing, diagnostics, and future media tooling.

## Tracking Rule

This README should be updated whenever implementation meaningfully changes so the repo has a current human-readable progress log alongside the specs and code.
