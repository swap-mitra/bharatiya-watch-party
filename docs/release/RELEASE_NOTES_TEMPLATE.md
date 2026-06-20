# Release Notes Template

Copy this file into `docs/release/RELEASE_NOTES_<version>.md` for each
release. The release process in `docs/release/RELEASE.md` describes when to
fill it in and where to ship it.

---

# Bharatiya Watch Party <version>

Released on <YYYY-MM-DD>.

## Highlights

- <bullet 1>
- <bullet 2>
- <bullet 3>

## What's new

- <feature / change>
- <feature / change>

## Bug fixes

- <fix>
- <fix>

## Upgrade notes

- <any migration steps>
- <breaking changes>

## Known limitations

- **Unsigned build**: Release artifacts for Windows and macOS are not
  code-signed. Windows SmartScreen may warn on first launch; macOS Gatekeeper
  will require right-click → Open. See `docs/release/SUPPORT_MATRIX.md` for
  the full trust posture.
- **No DRM**: Encrypted HLS/DASH streams (Widevine, PlayReady, FairPlay) are
  not supported. Use direct progressive or unencrypted HLS/DASH URLs.
- **No auto-updater**: There is no signed update channel. Users re-download
  the release artifact to upgrade.
- **In-memory rooms**: The signaling service holds room state in memory. A
  service restart drops active rooms and chat history.
- **No WebRTC media relay**: The transport is WebSocket signaling plus
  direct client media fetch. Latency-sensitive peers may still see drift
  during the host's heartbeat window.
- **Browser fallback is a smoke-test path**: DASH and libmpv-only features
  require the native backend. The fallback only supports MP4/WebM and
  browser-native HLS.

## Verification

The following commands passed on the release commit. Each release runs them
in CI before artifacts are published.

```powershell
cargo fmt --all -- --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
npm run typecheck
npm run lint
npm run desktop:build
```

## Artifacts

| Platform | File                                | Notes                                |
| -------- | ----------------------------------- | ------------------------------------ |
| Windows  | `Bharatiya-Watch-Party_<version>_x64-setup.exe` | NSIS installer (unsigned).    |
| Windows  | `Bharatiya-Watch-Party_<version>_x64_en-US.msi` | MSI installer (unsigned).     |
| macOS    | `Bharatiya-Watch-Party_<version>_x64.app`       | App bundle (unsigned).        |
| macOS    | `Bharatiya-Watch-Party_<version>_x64.dmg`       | DMG installer (unsigned).     |
| macOS    | `Bharatiya-Watch-Party_<version>_aarch64.app`   | Apple Silicon bundle (unsigned). |
| macOS    | `Bharatiya-Watch-Party_<version>_aarch64.dmg`   | DMG installer (unsigned).     |

The signal service is shipped as a single binary:

| Platform | File                                     |
| -------- | ---------------------------------------- |
| Windows  | `signal-service-<version>-x86_64-pc-windows-msvc.zip` |
| macOS    | `signal-service-<version>-x86_64-apple-darwin.tar.gz` |
| macOS    | `signal-service-<version>-aarch64-apple-darwin.tar.gz` |
| Linux    | `signal-service-<version>-x86_64-unknown-linux-gnu.tar.gz` |

## Source

Tarball: `bharatiya-watch-party-<version>.tar.gz`
Commit: <git-sha>

## Thanks

- <contributors>
