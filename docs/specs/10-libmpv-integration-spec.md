# libmpv Integration Spec

## Goal

Replace the current player harness with a real `libmpv` integration while preserving the public `PlayerAdapter` contract.

## Integration Boundary

- Rust owns the player process or embedded instance lifecycle
- The `PlayerAdapter` trait remains the stable internal contract
- Tauri commands and events remain the stable UI-facing contract

## Required Capabilities

- Load direct media URLs
- Play, pause, seek, stop
- Query current state and playback position
- Enumerate audio tracks
- Enumerate subtitle tracks
- Select audio track
- Select subtitle track or disable subtitles
- Surface native playback errors

## Embedding Strategy

- Use a native window handle or child surface compatible with Tauri on macOS and Windows
- Avoid browser-based playback fallback in the desktop app
- Keep the player rendering path separate from the React UI chrome

## Event Model

- Emit normalized state updates from native callbacks or polling
- Emit track catalog updates when source metadata is ready
- Emit error events with a user-safe message and a developer diagnostic payload

## Packaging Requirements

- Bundle or document `libmpv` runtime requirements for Windows and macOS
- Define how native libraries are discovered in development and release builds
- Ensure release artifacts include all required native assets

## Failure Handling

- Missing native library: fail fast with actionable startup guidance
- Unsupported stream: expose error state without crashing the app
- Track-selection failure: keep playback active and report the error

## Acceptance Criteria

- Real media plays through `libmpv` on macOS and Windows
- The existing Tauri command surface does not change
- Subtitle and audio-track switching works when the source exposes tracks
