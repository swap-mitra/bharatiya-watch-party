# Performance And Load Spec

## Goal

Define measurable performance expectations through Sprint 7 and Sprint 8.

## Key Budgets

- Room creation should feel immediate on a normal network
- Join flow should complete within a short interactive window
- Playback commands should fan out quickly to all connected participants
- Desktop shell should remain responsive while the player is active

## Room Load

- Support one host plus up to 10 viewers
- Presence, chat, and playback traffic must remain stable at full room size
- The service must not degrade sharply under repeated seek or chat bursts

## Desktop Performance

- Player UI controls should update without visible lag
- Theater mode toggle should not stall the main interface
- Chat scrolling should remain smooth during active playback

## Stress Scenarios

- Rapid host seeks
- Host pause/play spam
- Full room chat bursts
- Viewer disconnect and reconnect churn

## Measurement Plan

- Use synthetic room sessions for backend event fanout
- Use manual and scripted desktop runs for UI responsiveness and playback command latency
- Record measured values in the README or release notes once collected

## Acceptance Criteria

- Full room size remains usable
- Command fanout remains predictable under bursty host control behavior
- No single routine room action causes major UI or backend stalls
