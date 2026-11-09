# Global Roster Manager API

Rust 2024, Axum, PostgreSQL, Diesel Async, and Swagger UI. Implements the roster contract in [API.md](API.md) under `/api/v1`.

## Run

1. Create a PostgreSQL database.
2. Copy `.env.example` to `.env` and set `DATABASE_URL`, a random `JWT_SECRET` of at least 32 bytes, and `CLIENT_ORIGIN` (comma-separated frontend origins).
3. Set `ADMIN_EMAIL`, `ADMIN_NAME`, and `ADMIN_PASSWORD` (at least 12 characters) to bootstrap the first owner. An existing account is never overwritten. Remove the bootstrap password from the environment after provisioning.
4. Run `cargo run`. Migrations run automatically. The default listen address is `0.0.0.0:9000`.

Swagger: [http://localhost:9000/swagger-ui](http://localhost:9000/swagger-ui). OpenAPI: [http://localhost:9000/api-docs/openapi.json](http://localhost:9000/api-docs/openapi.json).

## Authentication

`POST /api/v1/auth/login` accepts `{ "email", "password", "remember": true }` and returns `{ "accessToken", "refreshToken", "expiresIn", "user" }`.

Send `Authorization: Bearer <accessToken>` on admin requests. Cookies are neither accepted nor issued. Send refresh tokens in the JSON body of `/auth/refresh`; each successful refresh rotates the token. Refresh-token replay is rejected. Logout revokes the session, including its access token. Password reset revokes every admin session. Access tokens last one hour; refresh sessions last 12 hours, or 30 days with `remember: true`.

Team authentication uses a separate JWT purpose and six-digit one-time codes. Team tokens cannot call admin endpoints. Roles and coordinator scope are checked against the current database state on every request.

### Authentication delivery adapter

Configure a trusted SMS/WhatsApp/email adapter with `AUTH_DELIVERY_URL` and `AUTH_DELIVERY_SECRET`. The server sends an HTTP POST with `Authorization: Bearer <AUTH_DELIVERY_SECRET>` and JSON:

```json
{ "kind": "team-code", "recipient": "08031234567", "value": "123456" }
```

`kind` is `team-code` or `password-reset`; reset recipients are email addresses and `value` is the reset token. The adapter must deliver the message and return a successful HTTP status. Reset links should point to your frontend's reset form and carry this token. The request times out after 10 seconds. No code or reset token is returned by the API or logged. `/auth/forgot-password` always returns 204 for valid, non-rate-limited requests, including unknown accounts and delivery failures. Team-code requests require adapter configuration; delivery failure is logged without revealing membership in the response.

The repository includes a **local-only** Postman delivery simulator for tests. It does not send actual messages. Connect your production adapter before enabling team sign-in or password recovery.

## Structure

- `src/dto/roster.rs`: request DTOs and transport validation.
- `src/handlers/roster/`: explicit routes and domain-specific HTTP handlers.
- `src/services/roster/`: roster rules, venue template, read models, team scope, imports, exports, PDF rendering, and notification maintenance.
- `src/models/roster.rs`: parameterized persistence queries and database pagination.
- `src/auth/admin.rs`: JWT sessions, rotation, revocation, OTPs, and owner bootstrap.
- `src/errors/api.rs`: contract error envelopes.
- `src/documentation/`: OpenAPI specification and Swagger document loading.
- `migrations/2026-11-09-000000_roster/`: roster schema, indexes, foreign keys, and uniqueness constraints.

The existing template modules remain in place but their unrelated routes are not mounted. JSON documents preserve the contract's camelCase fields; relational foreign keys and indexed columns enforce event/zone/position/member relationships. Unique constraints enforce one assignment per session/member, one active occupant per seat, and one lead per zone. Session-level transactional locks serialize roster edits and replacement operations. Member lists are filtered, sorted, and paginated in PostgreSQL.

The venue template follows the explicit door list in API.md: 17 doors (D1–D9, X1–X5, M1–M3), despite the prose saying 19. This gives 121 zones and 312 usher seats. Session times are configurable; absence cutoff uses Nigeria's UTC+1 time zone.

CSV, XLSX, and XLS imports accept files up to 5 MB and 10,000 rows. Preview records expire after one hour and commits consume them once. CSV/XLSX/PDF exports omit unavailable ushers from active rosters; PDFs use A4 pages, wrapped cells, repeated headings, and embedded Noto Sans (SIL OFL, included with the font).

Notifications are maintained every 15 minutes and refreshed on the notification feed. Expired sessions, codes, previews, and rate-limit records are cleaned up periodically. The demo-reset route is registered **only** with `APP_ENV=development`. Default logs exclude SQL parameters.

## Checks

```sh
cargo fmt --check
cargo check --offline
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
```

For end-to-end verification, use a **disposable** PostgreSQL database, start the local delivery simulator, and start the actual Rust server with the test configuration:

```sh
postman simulation run postman/simulations/auth-delivery/auth-delivery.sim.yaml --port 55560 --no-history --output ndjson
```

In a second terminal:

```sh
DATABASE_URL=postgres://localhost/tga_roster_test \
JWT_SECRET=local-integration-test-secret-over-32-bytes \
ADMIN_EMAIL=admin@example.test \
ADMIN_PASSWORD=local-test-password-123 \
ADMIN_NAME=Test \
AUTH_DELIVERY_URL=http://127.0.0.1:55560/auth-delivery/messages \
APP_ENV=production cargo run
```

Then run `python3 tests/api_integration.py`. The suite creates and removes fixture records, changes settings, exercises imports and exports, simulates concurrent seat claims, and resets the test owner's password to its original fixture value. Never run it against a production database. The simulator validates the local adapter request flow; it does not establish delivery through a real provider.

When running behind a reverse proxy, preserve the peer IP using a trusted ingress configuration. The server rate-limits the connection's peer address and deliberately does not trust arbitrary forwarded-IP headers.
