# Configuration Reference

The desktop app and signaling service both read configuration from
environment variables. This is the single source of truth for which variables
are honored, what their defaults are, and how they interact.

## Desktop app

The desktop app is configured at **build time** through Vite environment
variables. They are baked into the JavaScript bundle during `npm run build` /
`tauri build`.

| Variable                       | Default              | Description                                                                                       |
| ------------------------------ | -------------------- | ------------------------------------------------------------------------------------------------- |
| `VITE_SIGNAL_SERVICE_URL`      | `http://127.0.0.1:4000` | Base URL for the HTTP routes (`/api/rooms`, `/api/rooms/{code}/join`).                            |
| `VITE_SIGNAL_SERVICE_WS_URL`   | `ws://127.0.0.1:4000`   | Base URL for the WebSocket route (`/ws`).                                                          |
| `MPV_LIBRARY_PATH`             | unset                | Runtime override for the libmpv search path. The loader tries this exact path first.              |

Example for a production build pointed at a deployed signaling service:

```powershell
$env:VITE_SIGNAL_SERVICE_URL = "https://signal.example.com"
$env:VITE_SIGNAL_SERVICE_WS_URL = "wss://signal.example.com"
npm run desktop:build
```

The desktop app never embeds secrets. URLs are the only build-time
configuration; there is no API key, license key, or signing certificate.

## Signaling service

The service reads configuration at **runtime** from environment variables.
See `docs/release/SIGNAL_SERVICE_DEPLOYMENT.md` for the full table. Quick
reference:

| Variable                    | Default       | Description                                                          |
| --------------------------- | ------------- | -------------------------------------------------------------------- |
| `BIND_ADDR`                 | `0.0.0.0:4000`| Bind address for HTTP and WebSocket traffic.                          |
| `ROOM_TTL_SECONDS`          | `14400`       | Idle room TTL (4 hours by default).                                  |
| `DISCONNECT_GRACE_SECONDS`   | `60`          | Host reconnect grace before the room is closed.                      |
| `RUST_LOG`                  | `info`        | Structured log filter applied via `tracing-subscriber`.              |
| `STUN_URLS`                 | unset         | Reserved; parsed but ignored in v1.                                  |
| `TURN_URLS`                 | unset         | Reserved; parsed but ignored in v1.                                  |
| `TURN_USERNAME`             | unset         | Reserved; parsed but ignored in v1.                                  |
| `TURN_CREDENTIAL`           | unset         | Reserved; parsed but ignored in v1. Never logged or persisted.       |
| `CORS_ALLOWED_ORIGINS`      | dev defaults  | Comma-separated additional CORS origins for the WebView and clients. |

## Dev vs production configs

The repository intentionally keeps dev and production on the same code path.
The differences are purely environmental:

| Concern               | Dev (local)                                | Production                                                                |
| --------------------- | ------------------------------------------ | ------------------------------------------------------------------------- |
| Signal service URL    | `http://127.0.0.1:4000` / `ws://127.0.0.1:4000` | `https://signal.example.com` / `wss://signal.example.com` over TLS proxy |
| Tauri dev URL         | `http://localhost:1420`                    | `frontendDist: "../dist"`                                                 |
| Tauri CORS            | Tauri dev origins + `localhost:1420`       | Restricted via `CORS_ALLOWED_ORIGINS` and reverse proxy                   |
| `RUST_LOG`            | `info` (default)                           | `info` or `warn` in production; never `debug`                              |
| Code signing          | Disabled (unsigned)                        | Disabled (unsigned)                                                       |
| Secrets in repo       | None                                       | None                                                                      |
| GitHub Actions secrets | None required                              | None required                                                             |

Production configs are documented but never committed. Operators set
environment variables on the host or in a `.env` file that is git-ignored.
