# Product Spec

## Goal

Ship a desktop-first watch party app for private rooms where a host shares a direct media stream URL and viewers watch in near real time with low-latency text chat.

## Audience

- Small friend groups
- Invite-only rooms
- Low-friction join flow without accounts

## v1 Scope

- macOS and Windows desktop app
- Host creates a room and receives a short room code
- Up to 10 viewers can join with room code plus display name
- Host pastes a direct non-DRM media URL
- Host-only playback control
- Text chat only
- Ephemeral rooms and chat history
- Standard mode with side chat
- Theater mode with chat below the player

## Success Criteria

- Room creation to ready state completes quickly on a typical home network
- Viewer playback stays closely aligned with host playback
- Room full, invalid stream, and expired room states are clear and deterministic
- Join and playback UI remain compact and intuitive

## Explicit Non-Goals

- DRM-protected streams
- Public rooms
- User accounts or identity federation
- Voice or video chat
- Long-term room history
- Browser, iOS, or Android clients in v1