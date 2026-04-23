# Observability Implementation Spec

## Goal

Make room lifecycle, playback sync, reconnects, and failures observable enough to debug production issues.

## Logging

- Structured logs in Rust services
- Log room create, join, connect, disconnect, room close, and playback command events
- Include room code and session ID where safe and useful
- Log player adapter failures with normalized error categories

## Metrics

- Room creation count
- Room join count
- Active room count
- Active participant count
- Playback command count
- Chat message count
- Reconnect count
- Room close reasons

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
