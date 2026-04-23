# Player Spec

## Decision

Use `libmpv` as the primary playback engine for desktop-native playback. Use `FFmpeg` only as a supporting utility for probing, diagnostics, or future remux/transcode helpers.

## Required Player Contract

- `load_stream`
- `play`
- `pause`
- `seek`
- `stop`
- `state`
- `tracks`
- `select_audio_track`
- `select_subtitle_track`

## Source Support

- HLS manifests
- DASH manifests
- MP4
- MKV and WebM where supported by the underlying decoder stack
- Common direct audio streams

## Output Requirements

- Player state events must be emitted to the UI
- Audio and subtitle tracks must be enumerable when present
- The player contract must work the same way on macOS and Windows
- The desktop shell must expose the same command and event surface regardless of the underlying native player binding

## Integration Strategy

- The desktop app owns the player lifecycle through a Rust `PlayerAdapter` trait
- Tauri commands and events bridge player state to the React shell
- The first implementation may be a harness or stub, but the public contract must remain stable when the real `libmpv` integration is added

## Non-Goals

- DRM playback in v1
- Site-page URL scraping or resolution in v1
- Browser-based playback as the primary architecture
