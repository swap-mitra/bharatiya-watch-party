# Bharatiya Watch Party

Cross-platform desktop watch party app for macOS and Windows with a Rust core, WebSocket signaling, and a desktop UI shell built with Tauri, React, and TypeScript.

## Workspace

- pps/desktop: desktop shell and frontend harness
- crates/app-core: shared domain types, validation, protocol, player contract
- services/signal-service: Rust signaling and ephemeral room service
- docs/specs: product and architecture specs

## Current Scope

- Sprint 1: repo scaffold, specs, shared domain types
- Sprint 2: signaling service and room lifecycle
- Sprint 3: desktop shell and native player adapter interface

## Notes

- Playback is designed around libmpv; FFmpeg is a secondary utility option, not the primary player surface.
- The repo currently ships the player contract and desktop harness needed to continue native playback integration.