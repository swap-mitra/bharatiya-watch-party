# Realtime Protocol Spec

## Authority Model

- Host is authoritative for playback state
- Viewers may send readiness, presence-affecting lifecycle events, and text chat
- Viewer playback mutation attempts must be rejected deterministically

## Transport

- HTTP is used for room creation and viewer reservation
- WebSocket is used for session attachment and realtime room traffic
- Media is not transported through the signaling service

## HTTP Contracts

### `POST /api/rooms`

- Request: `CreateRoomRequest`
- Response: `CreateRoomResponse`
- Creates a room, reserves the host session, and returns a room code and session ID

### `POST /api/rooms/{room_code}/join`

- Request: `JoinRoomRequest`
- Response: `JoinRoomResponse`
- Reserves a viewer slot and returns a viewer session ID plus the current room snapshot

## WebSocket Attachment

- Endpoint: `/ws?room_code={room_code}&session_id={session_id}`
- A session may connect only if it was previously created or reserved through HTTP
- Reconnect uses the same `session_id`

## Client Messages

- `Ping`
- `ReadyState { ready }`
- `ChatSend { text }`
- `PlaybackCommand(PlaybackCommand)`

## Server Messages

- `Welcome { room, playback, self_session_id }`
- `Presence(RoomSnapshot)`
- `Chat(ChatMessage)`
- `Playback(PlaybackCommand)`
- `Error { code, message }`
- `RoomClosed { reason }`

## Playback Ordering

- Playback commands carry a monotonic `seq`
- The room stores `last_sequence`
- Commands with `seq <= last_sequence` are ignored as stale
- `stream_url` on playback commands must pass stream URL validation before broadcast

## Error Model

- Bad message payloads yield `Error { code: "bad_message" }`
- Rejected authorized-but-invalid actions yield `Error { code: "message_rejected" }`
- Unknown rooms, invalid sessions, and room-full cases are handled at the HTTP boundary before WebSocket attachment

## Lifecycle Rules

- Host disconnect closes the room and broadcasts `RoomClosed { HostDisconnected }`
- Viewer disconnect removes the active connection and updates presence
- Expired rooms broadcast `RoomClosed { Expired }` when they are swept while clients are connected
