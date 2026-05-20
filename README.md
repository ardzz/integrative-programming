# Blog API

[![CI](https://github.com/ardzz/integrative-programming/actions/workflows/ci.yml/badge.svg?branch=master)](https://github.com/ardzz/integrative-programming/actions/workflows/ci.yml)
[![Rust](https://img.shields.io/badge/rust-1.85%2B-orange?logo=rust)](https://www.rust-lang.org)
[![License](https://img.shields.io/badge/license-MIT-blue.svg)](LICENSE)

A RESTful blog API built with Rust, Axum, and MySQL. Outputs structured JSON logs compatible with [Gonzo](https://github.com/control-theory/gonzo) for real-time log analysis.

CI runs four parallel jobs on every push: `Format + Clippy`, `MSRV (Rust 1.85)`, `Cargo Deny (advisories + licenses + bans)`, and `Test (MySQL service container)`. See the [Actions tab](https://github.com/ardzz/integrative-programming/actions) for the live build status.

## Tech Stack

- **Rust** (edition 2021) + **Axum 0.8**
- **MySQL 8.0** via SQLx (compile-time checked queries)
- **JWT** authentication (jsonwebtoken + Argon2)
- **tracing** with JSON structured logging
- **Docker Compose** for local development

## Quick Start

```bash
cp .env.example .env
docker compose up -d mysql
cargo run
```

The API starts on `http://localhost:3000`.

## API Endpoints

| Method | Path | Auth | Description |
|--------|------|------|-------------|
| GET | /health | No | Health check |
| POST | /api/auth/register | No | Register user |
| POST | /api/auth/login | No | Login |
| GET | /api/users | Yes | List users |
| GET/PUT/DELETE | /api/users/{id} | Yes | User CRUD |
| GET/POST | /api/posts | Mixed | List (public) / Create (auth) |
| GET/PUT/DELETE | /api/posts/{id} | Mixed | Post CRUD |
| GET/POST | /api/posts/{post_id}/comments | Mixed | List (public) / Create (auth) |
| GET/PUT/DELETE | /api/posts/{post_id}/comments/{comment_id} | Mixed | Comment CRUD |

## Gonzo Integration

The API supports two log formats controlled by the `LOG_FORMAT` environment variable:

- **Pretty** (default) — human-readable colored output for local development
- **JSON** (`LOG_FORMAT=json`) — structured JSON output for Gonzo consumption

Docker Compose sets `LOG_FORMAT=json` automatically.

### Install Gonzo

```bash
# Homebrew
brew install gonzo

# Go install
go install github.com/control-theory/gonzo/cmd/gonzo@latest

# Or download from https://github.com/control-theory/gonzo/releases
```

### Usage

#### Pipe logs directly (cargo)

```bash
LOG_FORMAT=json cargo run 2>&1 | gonzo
```

#### With Docker Compose (JSON is automatic)

```bash
docker compose up api 2>&1 | gonzo
```

#### From Docker container logs

```bash
docker compose up -d
docker compose logs -f api | gonzo
```

#### Save to file, then analyze

```bash
LOG_FORMAT=json cargo run 2>&1 | tee app.log
gonzo -f app.log --follow
```

### Example JSON Log Output

```json
{"timestamp":"2025-04-15T10:30:00.123Z","level":"INFO","target":"blog_api","message":"listening on 0.0.0.0:3000"}
{"timestamp":"2025-04-15T10:30:01.456Z","level":"DEBUG","target":"tower_http::trace","message":"started processing request","http.method":"GET","http.uri":"/health"}
```

Gonzo auto-detects the JSON format and displays severity distribution, request patterns, and real-time charts in its TUI dashboard.

## Docker

```bash
# Full stack
docker compose up

# Build only
docker compose build api
```

## Rate Limiting

Rate limiting is controlled by `RATE_LIMIT_ENABLED`. When enabled, the API applies a global limit of 60 requests per minute and a stricter 5 requests per minute limit on the authentication endpoints `/api/auth/register`, `/api/auth/login`, and `/api/auth/refresh`.

The `/health` endpoint is exempt so health checks remain unaffected. Tests default to rate limiting disabled to avoid flaky integration runs.

## HTTPS Deployment

For production deployment, HTTPS termination is expected at the reverse-proxy layer such as Caddy, Nginx, or Traefik. The API itself continues to serve plain HTTP on port `3000`, while the reverse proxy handles TLS certificates and public HTTPS access. With Caddy, Let's Encrypt certificates are provisioned automatically.

```caddyfile
api.example.com {
    reverse_proxy blog-api:3000
}
```

## Token Refresh Flow

Use `POST /api/auth/refresh` to exchange a valid refresh token for a new access token without re-entering credentials.

```bash
curl -X POST http://localhost:3000/api/auth/refresh \
  -H "Content-Type: application/json" \
  -d '{"refresh_token":"<your-refresh-token>"}'
```

By default, access tokens expire after 15 minutes and refresh tokens expire after 7 days. You can override these values with `ACCESS_TOKEN_TTL_MINUTES` and `REFRESH_TOKEN_TTL_DAYS`.

## Chapter 4 Features

See [Laporan Praktikum Week4.md](../Laporan Praktikum Week4.md) for the Chapter 4 review summary.

- Pagination envelope (`{data, meta}`) on list endpoints.
- `403 Forbidden` semantics on owner-check failures.
- `/api/users/me` endpoints.
- Rate limiting.
- Stateless refresh token.

## Tests

```bash
cargo test
```

## Environment Variables

| Variable | Description | Default |
|----------|-------------|---------|
| `DATABASE_URL` | MySQL connection string | required |
| `JWT_SECRET` | Secret for JWT signing | required |
| `RATE_LIMIT_ENABLED` | Enable request rate limiting | `false` in tests / env-specific |
| `ACCESS_TOKEN_TTL_MINUTES` | Access token lifetime in minutes | `15` |
| `REFRESH_TOKEN_TTL_DAYS` | Refresh token lifetime in days | `7` |
| `RUST_LOG` | Log filter directive | `blog_api=debug,tower_http=debug` |
| `LOG_FORMAT` | Log output format (`json` or omit for pretty) | pretty |
