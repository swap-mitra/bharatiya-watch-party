# Sync Correction Spec

## Goal

Define how playback converges toward the host timeline once the app moves beyond command replication.

## Authority

- Host timeline is the source of truth
- Viewers follow host-issued commands and heartbeats
- Viewers never become timeline authority in v1

## Inputs

- Host playback commands
- Host playback heartbeats
- Local player position
- Local player status

## Correction Thresholds

- Small drift: no visible correction or very light adjustment
- Medium drift: micro seek or bounded correction
- Large drift: hard seek to host target position

Exact thresholds should be tuned during implementation and recorded after measurement.

## Join Behavior

- Late joiners receive current source, target position, and playback status
- Client loads the source, waits for metadata readiness, then seeks near host target
- Client begins playback only after minimum ready state is reached

## Pause / Seek / Stop Semantics

- Pause: all viewers converge to paused state quickly
- Seek: latest host seek wins; stale seeks are ignored
- Stop: viewers return to stopped state and reset active position

## Drift Loop

- Sync loop runs on a monotonic clock
- Sync loop is suspended when no active source exists
- Sync loop handles buffering and reconnect transitions explicitly

## Acceptance Criteria

- Late join lands close to host timeline
- Repeated seeks do not leave viewers permanently drifted
- Small transient jitter does not cause excessive visible jumping
