# libmpv Bundling

The desktop shell loads `libmpv` at runtime through the Rust `PlayerAdapter`
implemented in `apps/desktop/src-tauri/src/main.rs`. The dynamic loader tries
three sources, in order:

1. The exact path provided by the `MPV_LIBRARY_PATH` environment variable.
2. Platform-specific well-known names relative to the executable directory.
3. The system search path (`PATH` on Windows, `DYLD_LIBRARY_PATH` on macOS,
   `LD_LIBRARY_PATH` on Linux).

The well-known names the loader tries, in order, are:

| Platform | Candidate names                                                                 |
| -------- | ------------------------------------------------------------------------------- |
| Windows  | `mpv-2.dll`, `libmpv-2.dll`, `mpv-1.dll`                                         |
| macOS    | `libmpv.2.dylib`, `libmpv.dylib`                                                 |
| Linux    | `libmpv.so.2`, `libmpv.so`                                                       |

If every candidate fails, the desktop shell starts in **mock** mode and the
React UI receives a `backend: "mock"` bootstrap payload plus a warning. This is
the official developer fallback for smoke testing without native playback.

## Discovery at runtime

The loader is the source of truth for "where can I find libmpv?". There is no
hidden probing in the UI. The candidates are listed in the source for
reviewability and to keep platform behavior deterministic.

## Verifying libmpv is reachable

`scripts/check-libmpv.ps1` performs a dry-run of the same discovery rules the
runtime uses. It does not load the library — it just reports which candidate
Tauri would have tried first on the current machine.

```powershell
pwsh ./scripts/check-libmpv.ps1
```

A successful run prints the full path of the first matching candidate and exits
`0`. A failed run prints the candidate list and exits non-zero so CI can gate
release builds.

## Bundling for distribution

Sprint 8 keeps the bundling rules **deterministic and unsigned**:

1. The bundle script copies a known libmpv build into
   `apps/desktop/src-tauri/bin/<target-triple>/` next to `tauri.conf.json`.
2. Tauri's `bundle.resources` map copies that file into the application
   directory of every produced bundle (MSI, NSIS, APP, DMG).
3. The runtime's well-known-name lookup finds the library next to the
   executable without requiring `MPV_LIBRARY_PATH`.

Because the app is unsigned in v1, end users receive an unsigned `libmpv`
binary alongside the unsigned Tauri binary. The `MPV_LIBRARY_PATH` environment
variable remains available for users who want to override the bundled copy.

## Unsigned build policy

Per `docs/specs/17-packaging-and-release-spec.md`, unsigned developer builds
remain supported and are the default for Sprint 8. There are no signing
certificates or secrets in this repository, and no GitHub Actions secrets are
required to produce a release artifact.

If a future sprint introduces code signing, the following fields are
deliberately left at their `null` defaults so a signing pipeline can be wired
in without changing the bundle shape:

- `bundle.windows.certificateThumbprint`
- `bundle.windows.digestAlgorithm`
- `bundle.windows.timestampUrl`
- `bundle.windows.signCommand`
- `bundle.macOS.signingIdentity`
- `bundle.macOS.providerShortName`
- `bundle.macOS.entitlements`

## Operator runbook

| Symptom                                                  | Likely cause                                                              | Action                                                                                       |
| -------------------------------------------------------- | ------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------- |
| Bootstrap payload says `backend: "mock"`                 | libmpv not on `PATH`, not next to the executable, and `MPV_LIBRARY_PATH` unset. | Install libmpv, place it next to the binary, or set `MPV_LIBRARY_PATH` before launching.    |
| Bootstrap payload says `backend: "mock"` with a warning  | libmpv exists but failed to load (ABI mismatch, missing dependency).       | Rebuild against the same libmpv release; on Linux install `libmpv.so.2` from your distro.    |
| Browser fallback handles MP4 fine, DASH fails             | Browser fallback is enabled by design; DASH requires libmpv or MSE/DASH.   | Provide libmpv (preferred) or implement MSE/DASH in a future sprint.                         |
