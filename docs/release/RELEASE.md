# Release Process

This document describes how a Sprint 8 release is cut. It intentionally keeps
the shape simple and **unsigned** — there are no signing certificates, no
notarization step, and no GitHub Actions secrets to manage.

## Inputs

- A green `cargo test`, `cargo clippy`, `npm run lint`, `npm run typecheck`,
  and `npm run desktop:build` on the release commit.
- A version bump in three places (Cargo workspace, `tauri.conf.json`, root
  `package.json`).
- A filled-in `docs/release/RELEASE_NOTES_<version>.md` based on the
  template.
- A signed-off QA acceptance run against the matrix in
  `docs/QA_ACCEPTANCE_MATRIX.md`.

## Step 1 — Decide the version

The desktop app, signaling service, and shared `app-core` crate are released
together. Bump all three to the same SemVer `MAJOR.MINOR.PATCH`.

| File                                            | Field          |
| ----------------------------------------------- | -------------- |
| `Cargo.toml` `[workspace.package]`              | `version`      |
| `crates/app-core/Cargo.toml`                    | `package.version` |
| `services/signal-service/Cargo.toml`            | `package.version` |
| `apps/desktop/src-tauri/Cargo.toml`             | `package.version` |
| `apps/desktop/src-tauri/tauri.conf.json`        | `version`      |
| `package.json`                                  | `version`      |
| `apps/desktop/package.json`                     | `version`      |

## Step 2 — Cut release notes

Copy `docs/release/RELEASE_NOTES_TEMPLATE.md` to
`docs/release/RELEASE_NOTES_<version>.md`. Fill in the highlights, what's new,
bug fixes, and known limitations. Keep the **unsigned build** warning and the
DRM/no auto-updater entries verbatim.

## Step 3 — Run verification

```powershell
pwsh ./scripts/verify.ps1
```

`verify.ps1` runs the full Sprint 8 regression suite in order: `cargo fmt`,
`cargo clippy`, `cargo test`, `npm run typecheck`, `npm run lint`, and
`npm run desktop:build`. The exit code is `0` only if every step passes.

## Step 4 — Build desktop artifacts

On Windows:

```powershell
pwsh ./scripts/build-desktop.ps1 -Platform windows
```

On macOS:

```bash
./scripts/build-desktop.sh -p macos
```

`build-desktop.ps1` runs `npm run desktop:build` and writes the bundle
outputs to `apps/desktop/src-tauri/target/release/bundle/`. The full target
list (`nsis`, `msi` on Windows; `app`, `dmg` on macOS) is built because
`bundle.targets` is `all` in `tauri.conf.json`.

The script then runs `scripts/bundle-libmpv.ps1` to copy a known libmpv build
into the bundle resources directory so the dynamic loader finds it next to
the executable.

## Step 5 — Build signal service artifacts

On each target platform:

```powershell
cargo build --release -p signal-service
pwsh ./scripts/package-signal-service.ps1 -Platform <windows|macos|linux>
```

The packaging script zips/tars the binary and writes it next to the desktop
artifacts.

## Step 6 — Publish

1. Tag the commit: `git tag -a v<version> -m "Bharatiya Watch Party <version>"`.
2. Push the tag: `git push origin v<version>`.
3. Attach the artifacts listed in the release notes to the GitHub release.
4. Attach the rendered release notes (Markdown is fine).

There are no GitHub Actions secrets to configure. The release is built on a
trusted developer machine because Sprint 8 deliberately avoids code signing.

## Step 7 — Update tracking

- Update `docs/implementation-status.md` with the released version.
- Update `README.md` if there are user-facing changes (new config, new
  support, etc.).
- Close the Sprint 8 acceptance checklist in
  `docs/QA_ACCEPTANCE_MATRIX.md`.

## Rollback

Because artifacts are unsigned and there is no auto-updater:

1. Mark the release as **withdrawn** on the GitHub release page.
2. Cut a follow-up patch release that reverts or fixes the issue.
3. Notify users via the release notes that they should re-download.
