# Observability And Performance Spec

## Performance Priorities

- Fast room creation and join
- Low overhead signaling for up to 10 viewers
- Minimal UI latency between player commands and visible state updates
- Predictable desktop shell behavior under repeated command use

## Metrics To Add

- Room create latency
- Room join latency
- Connected participant count
- Playback command fanout timing
- Disconnect and reconnect counts
- Player command failures
- Stream validation failures

## Logging Baseline

- Structured Rust logs on the signaling service
- Startup log for service bind address
- Warnings for serialization failures on outbound messages
- Future work: room/session correlation IDs and drift metrics

## Current Gaps

- No metrics sink yet
- No tracing spans around room actions yet
- No drift measurement because the sync engine is not implemented yet
- No frontend telemetry pipeline yet

## Performance Constraints For v1

- Room state remains memory-resident
- Media playback is local to each client
- The service must not transcode, proxy, or relay media content
