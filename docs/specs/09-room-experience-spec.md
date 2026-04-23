# Room Experience Spec

## Goal

Define the end-to-end user experience for room creation, joining, lobby readiness, active watching, reconnects, and room closure through Sprint 8.

## Primary Flows

### Host Flow

- Open app
- Create room with display name
- Receive room code
- Enter direct media URL
- Wait in lobby while viewers join
- Start playback when ready
- Control play, pause, seek, stop, and source changes

### Viewer Flow

- Open app
- Enter room code and display name
- Join room lobby
- Mark ready
- Wait for host playback
- Watch in sync and use chat

## Screen States

- Welcome
- Create room
- Join room
- Lobby
- Active watch room
- Reconnecting
- Room full
- Invalid room code
- Invalid stream
- Room closed

## Layout Behavior

- Standard mode: player left, chat/presence right
- Theater mode: player full width, chat/presence below
- Top bar always shows room code, connection state, and role
- Footer shows compact diagnostics and event state

## Host Rules

- Host can change source and playback state
- Host can see readiness count before playback starts
- Host can end the room
- Host can continue playback even if some viewers are not ready

## Viewer Rules

- Viewer cannot issue authoritative playback commands
- Viewer can mark ready or not ready
- Viewer can chat while in lobby or active watch room
- Viewer sees disabled playback mutation controls

## Error Handling

- Room full: block join and explain viewer cap
- Invalid room code: reject before socket attach
- Invalid stream: host sees validation error and playback does not update
- Lost connection: move to reconnecting state and preserve session context when possible
- Room closed: show closure reason and exit actions

## Acceptance Criteria

- All core room states are reachable from the desktop UI
- Host and viewer affordances are visibly distinct
- Standard and theater modes preserve the same functional controls
