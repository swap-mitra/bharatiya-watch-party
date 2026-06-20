# Support Matrix

Sprint 8 certifies the Bharatiya Watch Party MVP on the platforms and stream
categories listed below. Anything outside this matrix is unsupported in v1.

## Operating systems

| Platform            | Versions                                | Architecture | Notes                                                                          |
| ------------------- | --------------------------------------- | ------------ | ------------------------------------------------------------------------------ |
| Windows             | Windows 10 (1809+), Windows 11          | x64          | Built and verified via Tauri MSI and NSIS installers. Unsigned by default.     |
| macOS               | 10.15 (Catalina) and later              | x64, arm64   | Built and verified via Tauri APP and DMG bundles. Unsigned by default.         |
| Linux (development) | Ubuntu 22.04 LTS, Fedora 38 (best effort) | x64        | Used for CI and local development. Not a release target.                       |

The minimum Windows 10 build is 1809 (October 2018 Update) because the desktop
shell depends on the WebView2 runtime, which Microsoft ships with the OS from
that build onward.

The minimum macOS release is 10.15 (Catalina). This is the oldest release that
Apple currently supports and matches Tauri's documented default.

## Stream categories

| Category                       | Status            | Notes                                                                              |
| ------------------------------ | ----------------- | ---------------------------------------------------------------------------------- |
| HTTP Live Streaming (HLS)      | Supported (libmpv)| Native playback via libmpv. Browser fallback works where the WebView supports HLS.  |
| Dynamic Adaptive Streaming (DASH) | Supported (libmpv) | Native playback via libmpv. The browser fallback does not implement MSE/DASH.      |
| MP4 (progressive)              | Supported         | Native playback via libmpv; browser fallback uses the embedded `<video>` element.  |
| WebM                           | Supported         | Same as MP4 via either backend.                                                    |
| Subtitle selection             | Supported (libmpv)| Native track catalog and selection through the desktop bridge.                      |
| Alternate audio track selection| Supported (libmpv)| Native track catalog and selection through the desktop bridge.                      |
| Direct progressive downloads   | Supported         | `https://example.com/video.mp4` and similar URLs work end-to-end.                  |
| Encrypted HLS (`#EXT-X-KEY`)   | Not supported     | No DRM client is integrated. Requires Widevine or equivalent.                       |
| Encrypted DASH (`ContentProtection`) | Not supported | No DRM client is integrated. Requires Widevine or equivalent.                       |
| Widevine / PlayReady / FairPlay | Not supported   | Out of v1 scope. Documented as a known limitation.                                  |
| WebRTC peer-to-peer transport  | Disabled in v1    | `webrtcEnabled: false` in `/networking`. Reserved for future sprints.              |

## Network conditions

| Condition                                  | Behavior                                                                       |
| ------------------------------------------ | ------------------------------------------------------------------------------ |
| Stable LAN / broadband                     | Designed for this scenario. Drift correction is barely visible.                |
| Moderate latency (50–250 ms RTT)           | Drift correction engages; viewers converge through rate smoothing and seek.   |
| Disconnect and reconnect                   | Reconnect uses the stable session id; chat history is replayed.                |
| Full-room load (1 host + 10 viewers)       | Backend fanout is verified; metrics confirm one command per viewer.            |

## Code signing and trust

| Concern                                  | Status          |
| ---------------------------------------- | --------------- |
| Windows Authenticode signing             | Disabled (unsigned developer build). |
| Windows SmartScreen reputation           | Not certified. Users may see a SmartScreen warning on first launch. |
| Windows MSI signing                      | Disabled. |
| Windows NSIS signing                     | Disabled. |
| macOS codesign                           | Disabled (`signingIdentity: null`). |
| macOS notarization                       | Disabled (`providerShortName: null`). |
| macOS Gatekeeper                          | Users must right-click → Open on first launch. |
| Auto-updater artifacts                    | Disabled (`createUpdaterArtifacts: false`). |

The release artifacts are explicitly unsigned. See
`docs/release/RELEASE_NOTES_TEMPLATE.md` for the per-platform warning text
that release notes must include.

## Out of scope for Sprint 8

- Mobile (iOS/Android) shells. The Tauri config and code are mobile-ready at a
  basic level but no acceptance matrix covers mobile.
- WebRTC media relay. The signaling service reports `webrtcEnabled: false`.
- Persistence of room state, chat history, or user accounts. The service is
  intentionally ephemeral.
- Production metrics export to an external sink.
- Frontend telemetry pipeline.
- Release-time code signing.
