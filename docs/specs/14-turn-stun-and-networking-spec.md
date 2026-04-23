# TURN, STUN, And Networking Spec

## Goal

Define the networking strategy through Sprint 7, including when WebRTC is used and what fallback behavior exists.

## v1 Baseline

- HTTP + WebSocket signaling is required
- Media is always fetched directly from the source by each client
- The room service does not proxy or relay media

## WebRTC Position

- WebRTC is not required for the current foundation
- If adopted later, it should carry low-latency peer data where it provides measurable value
- WebRTC adoption must not become a prerequisite for basic room creation, chat, or playback sync

## STUN / TURN Scope

- STUN and TURN become relevant only when WebRTC transport is introduced
- TURN must be treated as a fallback path, not the primary network model
- TURN credentials and relay usage must be configurable per environment

## Fallback Strategy

- If peer transport is unavailable or unreliable, use hosted signaling transport for room events
- Chat, presence, and playback authority must continue working without peer transport

## Network Failure Modes

- Signaling loss
- Peer connection loss
- High-latency command delivery
- Partial room connectivity

## Acceptance Criteria

- The product remains functional on plain WebSocket signaling alone
- Future WebRTC adoption does not change the room-service media model
- Fallback behavior is explicit and testable
