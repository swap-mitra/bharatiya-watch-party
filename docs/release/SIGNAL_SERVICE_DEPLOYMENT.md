# Signal Service Deployment

The `signal-service` binary is the host-authoritative room controller. It
holds ephemeral room state, fans out playback commands, and exposes
`/health`, `/metrics`, and `/networking`. This document captures how it is
deployed, separately from the desktop app.

## Why a separate deployment

- The desktop app is downloaded and run by each participant. The signaling
  service is a single shared backend that coordinates them.
- The two have different update cadences and different failure domains. The
  desktop app must keep working with an old backend while a new build rolls
  out; the backend must keep working while clients upgrade.
- Splitting the deployment keeps resource sizing, log retention, and metrics
  export independent of the desktop bundle.

## Configuration

The service reads its operational config from environment variables. Defaults
match local development; production deployments should set every variable
explicitly.

| Variable                  | Default              | Description                                                                                            |
| ------------------------- | -------------------- | ------------------------------------------------------------------------------------------------------ |
| `BIND_ADDR`               | `0.0.0.0:4000`       | Socket address the HTTP and WebSocket servers bind to.                                                |
| `ROOM_TTL_SECONDS`        | `14400`              | Seconds before an idle room is eligible for expiration. Defaults to 4 hours.                           |
| `DISCONNECT_GRACE_SECONDS`| `60`                 | Seconds the host's session stays attached after a disconnect before the room is closed.                |
| `RUST_LOG`                | `info`               | Standard `tracing-subscriber` filter applied to the structured logs.                                   |
| `STUN_URLS`               | unset                | Reserved for future WebRTC signaling. Parsed and ignored in v1.                                        |
| `TURN_URLS`               | unset                | Reserved for future WebRTC signaling. Parsed and ignored in v1.                                        |
| `TURN_USERNAME`           | unset                | Reserved for future WebRTC signaling.                                                                  |
| `TURN_CREDENTIAL`         | unset                | Reserved for future WebRTC signaling. Secrets only; never commit to source control.                    |
| `CORS_ALLOWED_ORIGINS`    | Tauri local dev only | Comma-separated list of additional origins permitted for CORS. Production builds should set this explicitly. |

Defaults are conservative and intended for local development. Production
deployments are expected to lock down `BIND_ADDR`, restrict `CORS_ALLOWED_ORIGINS`
to the desktop identifier origin(s), and disable any future STUN/TURN that has
not been reviewed.

## Local development

```powershell
cargo run -p signal-service
```

The service listens on `0.0.0.0:4000` and accepts WebSocket and HTTP traffic
from `localhost:1420`, `127.0.0.1:1420`, and the Tauri WebView origins used
by `tauri dev`/`tauri build`. The desktop app reads `VITE_SIGNAL_SERVICE_URL`
and `VITE_SIGNAL_SERVICE_WS_URL`; both default to `127.0.0.1:4000`.

## Production deployment

The service is intentionally a single static binary with no embedded
secrets and no persistent storage. A minimal production shape:

```text
signal-service --bind 0.0.0.0:4000
```

Behind a TLS-terminating reverse proxy that:

- Terminates TLS for the public hostname (for example, `signal.example.com`).
- Forwards `/api/rooms`, `/api/rooms/{code}/join`, `/ws`, `/health`,
  `/metrics`, and `/networking` to the local service.
- Restricts `/metrics` to internal scrapers (do not expose publicly without a
  scraper-side allowlist).
- Optionally sets `BIND_ADDR` to `127.0.0.1:4000` so the service never binds
  to the public interface directly.

The desktop app is built with the matching `VITE_SIGNAL_SERVICE_URL` /
`VITE_SIGNAL_SERVICE_WS_URL` for the deployment. For example:

```powershell
$env:VITE_SIGNAL_SERVICE_URL = "https://signal.example.com"
$env:VITE_SIGNAL_SERVICE_WS_URL = "wss://signal.example.com"
npm run desktop:build
```

## Network posture

`GET /networking` always reports:

```json
{
  "signalingTransport": "websocket",
  "mediaTransport": "direct-client-fetch",
  "webrtcEnabled": false,
  "stunConfigured": false,
  "turnConfigured": false,
  "fallbackTransport": "hosted-websocket-signaling"
}
```

`webrtcEnabled`, `stunConfigured`, and `turnConfigured` will move to `true` in
future sprints once signaling and config wiring land. v1 deliberately keeps
WebRTC disabled so release artifacts do not promise features the backend
does not yet support.

## Health and metrics

- `GET /health` → `{ "ok": true }`
- `GET /metrics` → `ServiceMetricsSnapshot` (room, transport, chat,
  playback, validation, outbound, fanout counters).
- `GET /networking` → `NetworkingSnapshot` (see above).

These endpoints are unauthenticated by design for v1 because they expose no
sensitive data. Production deployments are expected to firewall them or place
them behind the proxy described above.

## Secret handling

- `TURN_CREDENTIAL` and any future media-relay credentials are the only
  secrets this service will ever read.
- They are read from environment variables and never logged, never written to
  disk, and never embedded in release artifacts.
- The repository contains no signing certs, no API keys, and no embedded
  secrets. CI builds use no GitHub Actions secrets for the Sprint 8 release
  shape.
