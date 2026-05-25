# Observability Implementation Spec

## Goal

Make room lifecycle, playback sync, reconnects, and failures observable enough to debug production issues.

## Logging

- Structured logs in Rust services
- Log room create, join, connect, disconnect, room close, and playback command events
- Include room code and session ID where safe and useful
- Log player adapter failures with normalized error categories

## Implemented Logging Baseline

- Signaling service logs room create, room join, WebSocket connect, reconnect, disconnect, room close, accepted chat messages, accepted playback commands, HTTP create/join latency, and playback fanout completion
- Outbound serialization failures are logged as warnings
- Logs remain console-based in development

## Metrics

- Room creation count
- Room join count
- Active room count
- Active participant count
- Playback command count
- Chat message count
- Reconnect count
- Room close reasons

## Implemented Metrics Baseline

- `RoomRegistry::metrics_snapshot()` returns in-process counters plus active room and participant gauges
- `GET /metrics` exposes the same snapshot as JSON for local diagnostics and later scraping
- Playback fanout count, total milliseconds, and max milliseconds are tracked for host command/heartbeat broadcasts
- Unauthorized, validation, stream validation, outbound send, and outbound failure counters are tracked

## Sync-Specific Metrics

- Host-to-viewer drift samples
- Late-join catch-up time
- Correction frequency
- Rebuffer incidents during synced playback

## Frontend Telemetry

- Track UI transition failures
- Track room connection failures
- Track player command failures surfaced to the client

## Operational Outputs

- Development logs in console
- Production sink to be chosen later, but interface should assume exportable metrics and structured events

## Acceptance Criteria

- A failed room session can be reconstructed from logs
- Core room lifecycle and sync events are measurable
