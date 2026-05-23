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

- Paused drift above 500 ms: seek to the host paused position
- Playing drift below 250 ms: no correction and reset speed to 100% if needed
- Playing drift from 250 ms to 2499 ms: smooth with bounded playback-rate correction
- Playing drift from 2500 ms to 4999 ms: seek to the host heartbeat position
- Playing drift at or above 5000 ms: hard seek to the host heartbeat position and log the correction

These thresholds are conservative first-pass values. They should be tuned after local multi-client testing and later replaced or augmented with playback-rate trimming.

## Join Behavior

- Late joiners receive current source, target position, and playback status
- Client loads the source, waits for metadata readiness, then seeks near host target
- Client begins playback only after minimum ready state is reached

## Pause / Seek / Stop Semantics

- Pause: all viewers converge to paused state quickly
- Seek: latest host seek wins; stale seeks are ignored
- Stop: viewers return to stopped state and reset active position

## Drift Loop

- Host emits heartbeat messages every 2000 ms while an active source is loading, playing, paused, or buffering
- Signal service accepts heartbeat messages from the host only and broadcasts accepted heartbeats to the room
- Viewers compare local player position to the host heartbeat position and correct by seeking when thresholds are exceeded
- Viewers use 97% or 103% playback speed to smooth medium drift before seeking
- Host commands and large corrections reset playback speed to 100%
- Sync loop should eventually run on a monotonic clock
- Sync loop is suspended when no active source exists
- Sync loop handles buffering and reconnect transitions explicitly

## Acceptance Criteria

- Late join lands close to host timeline
- Repeated seeks do not leave viewers permanently drifted
- Small transient jitter does not cause excessive visible jumping
