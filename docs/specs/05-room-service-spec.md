# Room Service Spec

## Responsibilities

- Create rooms
- Reserve host and viewer sessions
- Enforce room capacity
- Attach WebSocket sessions
- Broadcast presence, chat, playback, and room-close events
- Expire ephemeral rooms

## Storage Model

- In-memory room registry
- In-memory participant/session records
- In-memory playback snapshot per room
- No persistent chat or room history in v1

## Room Rules

- Exactly one host session per room
- Maximum of 10 viewer sessions per room
- Host plus viewers are represented in a single participant list
- Display names must be unique within a room, case-insensitively

## Session Rules

- Session IDs are generated server-side
- A session may reconnect to the same room using the original session ID
- Viewer reservations happen before WebSocket connection
- Unknown or unreserved session IDs are rejected

## Expiry

- Rooms have a fixed TTL
- Room expiry is extended on room activity
- Expired rooms are removed by a background sweep loop

## Broadcast Rules

- Presence snapshots are broadcast on connect, disconnect, and ready-state changes
- Chat messages are broadcast to all connected participants in the room
- Playback commands are broadcast only after host authorization and validation
- Room closure is broadcast to all connected viewers when the host disconnects

## Error Mapping

- Validation failures map to `400`
- Unknown room maps to `404`
- Room full maps to `409`
- Unauthorized action maps to `403`
- Missing participant/session maps to `401`
