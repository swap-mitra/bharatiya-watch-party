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

## Implemented Metrics Surface

- `GET /metrics` returns an in-memory service metrics snapshot
- Metrics include room creates, joins, active rooms, active connected participants, WebSocket connects, reconnects, disconnects, room closes, room expirations, playback commands, playback heartbeats, chat messages, unauthorized messages, validation failures, stream validation failures, outbound sends, outbound send failures, and playback fanout timing totals/max
- Metrics reset when the signaling service process restarts

## Logging Baseline

- Structured Rust logs on the signaling service
- Startup log for service bind address
- Warnings for serialization failures on outbound messages
- Room create, join, connect, disconnect, close, accepted chat, accepted playback command, and playback fanout logs include room/session context where useful
- Future work: durable trace export and deeper drift metrics

## Current Gaps

- No production metrics sink yet
- No distributed tracing backend yet
- Drift samples are not exported as service metrics yet
- No frontend telemetry pipeline yet

## Performance Constraints For v1

- Room state remains memory-resident
- Media playback is local to each client
- The service must not transcode, proxy, or relay media content
