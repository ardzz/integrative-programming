# BLOG API — PROJECT KNOWLEDGE BASE

**Generated:** 2026-04-21
**Commit:** d1d8079

## OVERVIEW

Rust 2021 + Axum 0.8 REST API. MySQL 8.0 via SQLx 0.8 (runtime queries, not compile-time macros). JWT auth (HS256, 24h expiry) + Argon2id password hashing. Docker Compose for local dev.

## STRUCTURE

```
blog-api/
├── src/              # Application source (see src/AGENTS.md)
├── tests/            # Integration tests (reqwest HTTP client)
│   ├── common/mod.rs # Shared harness: spawn_app(), helpers
│   ├── auth_test.rs  # 6 tests
│   ├── post_test.rs  # 9 tests
│   └── comment_test.rs # 8 tests
├── migrations/       # SQLx forward-only (.up.sql only, no rollback)
├── evidence/         # 50 PNGs for lab report (termshot captures)
├── scripts/          # capture_evidence.sh, capture_source.sh
├── .sqlx/            # SQLx offline cache (inert — runtime queries used)
├── Dockerfile        # Multi-stage: rust:1.94-slim → debian:bookworm-slim
└── docker-compose.yml # MySQL 8.0 + API service
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add endpoint | `src/route.rs` + `src/handler/` | Wire route, add handler |
| Add DB model | `src/model/` + `src/schema/` | Row type + response type + request DTOs |
| Change auth logic | `src/auth.rs` | JWT + Argon2 + AuthUser extractor |
| Fix error mapping | `src/error.rs` | AppError enum + IntoResponse + sqlx::Error mapping |
| Add migration | `migrations/` | Forward-only `.up.sql` files |
| Add test | `tests/` | Uses `common::spawn_app()` harness |

## CONVENTIONS

- **No `Arc<AppState>`** — Axum's `with_state()` handles cloning; `MySqlPool` is cheaply cloneable
- **Inline SQL** — `sqlx::query`/`query_as` in handlers directly; no repository layer
- **Owner-only auth** — handlers check `auth.user_id != resource.user_id`; no RBAC — returns 403 Forbidden (was 401 Unauthorized in Week 3, migrated in Week 4)
- **/me endpoints** — self-actions use `/api/users/me` (IDOR mitigation); `/api/users/{id}` supports GET only.
- **PUT = full replace** — no PATCH endpoints
- **Hard deletes** — CASCADE on FK (delete user → delete posts → delete comments)
- **Error format** — always `{"error": "message"}` JSON
- **Post status** — ENUM `draft` | `published`, defaults to `draft`
- **Test naming** — `test_{action}_{condition}_returns_{status_code}`
- **Test isolation** — UUID-based unique emails, no DB cleanup between tests

## ANTI-PATTERNS (THIS PROJECT)

- Do NOT wrap `AppState` in `Arc` (comment in `src/lib.rs` explains why)
- Do NOT use `query!` macro — project uses runtime `query`/`query_as` (`.sqlx/` is inert)
- Do NOT add down migrations — project convention is forward-only
- Do NOT use Axum 0.7 path syntax (`:id`) — this is Axum 0.8 (`{id}`)

## COMMANDS

```bash
cargo run                     # Start server on :3000 (needs DATABASE_URL)
cargo test                    # Integration + unit tests (needs MySQL)
cargo clippy -- -D warnings   # Lint (zero warnings policy)
docker compose up             # Full stack (MySQL + API)
docker compose up --build     # Rebuild + start
bash scripts/capture_evidence.sh  # Screenshot all API endpoints
bash scripts/capture_source.sh    # Screenshot all source files
cargo test --test rate_limit_test -- --ignored --test-threads=1   # Rate limit tests (isolated)
```

## NOTES

- Port: always 3000 (hardcoded in main.rs and docker-compose.yml)
- Seed data: Alice (`alice@example.com`) + Bob (`bob@example.com`), password `qwerty`
- `LOG_FORMAT=json` enables structured JSON logging (set automatically in Docker)
- Docker build uses `SQLX_OFFLINE=true` but `.sqlx/` cache is empty — this is benign since no `query!` macros are used
- Tests require a live MySQL; `DATABASE_URL` from `.env` is shared between dev and test
- Week 4 env vars: `RATE_LIMIT_ENABLED`, `ACCESS_TOKEN_TTL_MINUTES`, `REFRESH_TOKEN_TTL_DAYS`.
