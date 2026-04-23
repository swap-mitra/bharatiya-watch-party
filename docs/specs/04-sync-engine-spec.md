# Sync Engine Spec

## Goal

Keep all viewers aligned to the host timeline with a host-authoritative model and minimal visible correction drift.

## v1 Authority

- Host chooses the active source
- Host controls play, pause, seek, and stop
- Viewers never originate timeline-changing commands

## Core State

- Active stream URL
- Playback status
- Current host position in milliseconds
- Last accepted playback sequence
- Participant readiness state

## v1 Synchronization Flow

- Host issues a playback command with a new monotonic sequence number
- Signaling service validates the sender and broadcasts the command
- Each viewer applies the command locally to its player surface
- A late joiner receives the current `Welcome` snapshot and starts from the latest known room playback state

## Correction Policy

- Sprint 3 foundation: command replication only, no drift correction loop yet
- Sprint 5 target: drift thresholds for no-op, small correction, and hard seek
- Sync math must use a monotonic clock in Rust when the drift engine is implemented

## Readiness Model

- Each participant can mark itself ready or not ready
- Presence snapshots expose readiness state
- The host can use readiness state to decide when to start playback, but v1 does not block playback automatically

## Non-Goals For The Current Foundation

- Playback-rate trimming
- Adaptive drift correction
- Buffer-aware delayed start
- TURN-assisted media relay
