# Session And Reconnect Spec

## Goal

Define deterministic session lifecycle and reconnect behavior for the signaling service and desktop client.

## Session Model

- Session IDs are created server-side during room creation or viewer reservation
- A session is scoped to a single room
- A session can reconnect while the room remains active

## Connection States

- Idle
- Connecting
- Connected
- Reconnecting
- Closed

## Heartbeat Policy

- Client sends periodic `Ping`
- Server tracks last-seen activity timestamp per connected session
- Inactive sessions are marked disconnected after timeout

## Reconnect Rules

- Client attempts reconnect with the same `session_id`
- If the room is still active and the session exists, reconnect succeeds
- If the host session reconnects, room authority is preserved
- If reconnect window expires or room is gone, client transitions to `Closed`

## Duplicate Connection Rules

- A second socket for the same session replaces the older active connection
- Presence remains tied to the session identity, not socket count
- Replaced sockets are closed cleanly when possible

## Viewer Disconnect Rules

- Viewer disconnect updates presence but does not close the room
- Viewer can reconnect within the allowed window

## Host Disconnect Rules

- Host disconnect starts a reconnect grace period in Sprint 5+
- If host fails to reconnect within the grace period, the room closes
- Before grace-period implementation, host disconnect closes the room immediately

## Acceptance Criteria

- Reconnect does not create duplicate participants
- Session identity remains stable across reconnect
- Presence reflects connection state transitions deterministically
