# QA And Acceptance Spec

## Goal

Define the certification matrix and acceptance gates needed to call Sprint 8 complete.

## Functional Acceptance

- Host can create a room
- Viewer can join a room with valid room code
- Room full behavior is enforced
- Chat works across connected participants
- Host-issued playback commands reach viewers
- Reconnect behavior follows the chosen reconnect policy

## Media Acceptance

- HLS playback
- DASH playback
- MP4 playback
- Subtitle selection when available
- Alternate audio-track selection when available

## Platform Acceptance

- Windows local build and packaged build
- macOS local build and packaged build

## Network Acceptance

- Stable LAN / broadband scenario
- Moderate-latency network scenario
- Disconnect and reconnect scenario
- Full room load scenario

## Regression Suite

- Rust unit and integration tests
- Frontend typecheck and lint
- Desktop production build
- Manual smoke test of create/join/watch/chat flow

## Exit Criteria For Sprint 8

- Specs through Sprint 8 are written
- Foundation and release-critical flows are implemented
- Verification commands pass
- Manual certification matrix is completed for supported platforms
