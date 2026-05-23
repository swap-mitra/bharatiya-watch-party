# Chat And Presence Spec

## Goal

Define realtime chat and participant presence behavior for the watch room.

## Chat Scope

- Text-only chat in v1
- Messages are ephemeral and room-scoped
- Chat is available in lobby and active watch states

## Message Rules

- Empty messages are rejected
- Messages exceeding the configured limit are rejected
- Clients attach a unique message id to each chat send
- The server suppresses duplicate message ids within a bounded recent-id window
- Messages are broadcast to all connected participants in the room
- Chat ordering follows server broadcast order
- The server keeps a bounded ephemeral chat history for reconnect and late-join replay

## Rendering Rules

- Messages show sender name and text
- Local optimistic rendering is optional; final displayed order follows confirmed server messages
- Clients merge incoming chat by message id to avoid duplicate display during reconnect
- Reconnecting and late-joining clients receive the recent bounded chat history in the welcome payload
- Chat history is cleared when leaving the room or when the room closes

## Presence Rules

- Presence list includes host and viewers
- Presence shows connected/disconnected state
- Presence shows ready/not-ready state
- Host is visually distinct from viewers

## Lobby Behavior

- Participants can mark ready or not ready
- Host can monitor ready count before starting playback
- Readiness changes are broadcast through presence snapshots

## Active Room Behavior

- Presence remains visible while watching
- Chat remains interactive in standard and theater layouts

## Acceptance Criteria

- Chat works for host and viewers in connected rooms
- Presence updates reflect ready-state and connection changes
- Duplicate chat sends with the same message id are not rebroadcast
- Reconnect welcome payloads include recent chat history
- Leaving a room clears local chat and presence state
