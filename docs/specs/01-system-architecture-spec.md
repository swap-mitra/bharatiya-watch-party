# System Architecture Spec

## Top-Level Components

- pps/desktop: Tauri desktop shell with React/TypeScript UI
- crates/app-core: shared Rust types for room lifecycle, protocol, validation, and player contracts
- services/signal-service: Rust xum service for room creation, join negotiation, signaling, and presence

## Media Model

- Media is never proxied through the backend
- Each client loads the host-selected direct media URL locally
- Playback synchronization happens through realtime control messages and heartbeats

## Network Model

- WebSocket signaling is always present
- P2P transport remains an extension point for later sprints
- Hosted fallback for non-media room events is acceptable
- STUN/TURN support is planned after the foundations are stable

## State Model

- Rooms are ephemeral and stored in memory in v1
- Host disconnect closes the room
- Viewer disconnect leaves the room active while the host remains connected
- Expired idle rooms are swept by the backend

## Security Baseline

- Random room codes
- Session tokens per participant
- Room capacity enforced server-side
- Input validation on names, room codes, URLs, and chat payloads