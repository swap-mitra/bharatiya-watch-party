# Packaging And Release Spec

## Goal

Define how the app is prepared, bundled, signed, and shipped by Sprint 8.

## Release Targets

- Windows desktop installer
- macOS desktop bundle and installer

## Native Dependencies

- Package `libmpv` runtime requirements with the app or provide deterministic discovery rules
- Ensure the signaling service deployment path is documented separately from the desktop app

## Build Outputs

- Desktop binaries and installers for each supported platform
- Versioned release artifacts
- Release notes with support matrix and known limitations

## Signing And Trust

- Windows signing requirements
- macOS signing and notarization requirements
- Unsigned developer builds remain supported for local testing

## Configuration

- Environment-driven signaling service URL
- Environment-driven optional STUN/TURN configuration for later phases
- Development and production configs must be documented separately

## Support Matrix

- Supported OS versions
- Supported stream categories
- Unsupported cases such as DRM

## Acceptance Criteria

- A new developer can run the app locally from documented commands
- Release artifacts are reproducible and platform-specific requirements are documented
