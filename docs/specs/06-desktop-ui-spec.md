# Desktop UI Spec

## Goal

Provide a minimal desktop watch surface that keeps the player central, exposes the native playback harness clearly, and reserves layout space for the watch-party chat experience.

## Visual Direction

- Dark, minimal, cinematic surface
- Compact top navigation
- Large central stage
- Sparse chrome and short copy
- No dashboard-card clutter

## Current Screens

- Desktop playback harness
- Player control strip
- Track selectors
- Side chat placeholder
- Theater mode layout toggle
- Event log and track summary footer

## Layout Rules

- Standard mode: player panel left, chat panel right
- Theater mode: player panel full width, chat panel moved below
- Footer remains compact and secondary to the player stage

## Interaction Rules

- Stream URL input accepts direct media URLs only
- Player commands map 1:1 to Tauri invoke commands
- State and track changes arrive through Tauri events
- Track selectors reflect the currently selected audio and subtitle tracks

## Reserved Future Screens

- Welcome screen with create/join room actions
- Room lobby
- Live room watch surface bound to realtime room state
- Reconnecting and room-closed states

## Accessibility Baseline

- Keyboard-accessible controls
- Clear focus order
- Compact but legible text sizing
- Distinct status labels for player state
