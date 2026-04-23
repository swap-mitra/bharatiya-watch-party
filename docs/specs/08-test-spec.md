# Test Spec

## Unit Coverage

### `app-core`

- Room code generation and parsing
- Display name validation
- Stream URL validation
- Client message serde round-trip
- Server message serde round-trip

### `desktop-shell`

- Player harness can load a stream
- Track catalog is exposed after stream load

## Integration Coverage

### `signal-service`

- Room full rejection at 10 viewers
- Unauthorized viewer playback command rejection
- Host disconnect closes the room
- Reconnect preserves session identity

## Verification Commands

- `cargo fmt --all`
- `cargo test --workspace`
- `npm run lint`
- `npm run typecheck`
- `npm run build --workspace @watchparty/desktop`

## Pending Test Areas

- HTTP route integration tests against the Axum router
- WebSocket session integration tests with real sockets
- Desktop command/event integration tests through Tauri
- Media format certification against a real `libmpv` binding
- Sync drift and late-join behavior after the sync engine is implemented
