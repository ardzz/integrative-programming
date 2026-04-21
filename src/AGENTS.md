# SRC — SOURCE CODE KNOWLEDGE BASE

**Generated:** 2026-04-21
**Commit:** d1d8079

## OVERVIEW

3-layer Axum application: handlers → models/schemas → database (inline SQL).

## STRUCTURE

```
src/
├── main.rs       # Bootstrap: env → tracing → DB pool → migrations → router → serve
├── lib.rs        # AppState(db, jwt_secret) + module exports
├── route.rs      # create_router(): all routes + CORS + tracing middleware
├── auth.rs       # JWT create/verify, Argon2 hash/verify, AuthUser extractor
├── error.rs      # AppError enum → IntoResponse (maps HTTP status codes)
├── handler/      # Request handlers (controller layer)
│   ├── auth.rs   # register(), login()
│   ├── user.rs   # list_users(), get_user(), update_user(), delete_user()
│   ├── post.rs   # CRUD + PostWithUser JOIN struct (private to handler)
│   └── comment.rs # CRUD + CommentWithUser JOIN struct + ensure_post_exists()
├── model/        # DB row types (sqlx::FromRow) + API response types (Serialize)
│   ├── user.rs   # UserRow → UserResponse (strips password field)
│   ├── post.rs   # PostRow → PostResponse (user_name=None without JOIN)
│   └── comment.rs # CommentRow → CommentResponse
└── schema/       # Request DTOs with validator::Validate derives
    ├── user.rs   # CreateUser, UpdateUser, LoginUser, AuthResponse
    ├── post.rs   # CreatePost, UpdatePost
    └── comment.rs # CreateComment, UpdateComment
```

## WHERE TO LOOK

| Task | Location | Notes |
|------|----------|-------|
| Add new entity | handler/ + model/ + schema/ + route.rs | Mirror existing user/post/comment pattern |
| Add auth to endpoint | handler fn signature | Add `auth: AuthUser` param (auto-extracts from Bearer token) |
| Change validation rules | schema/*.rs | `#[validate(...)]` attributes |
| Add error variant | error.rs | Add to `AppError` enum + match in `into_response()` |
| Change DB query | handler/*.rs | SQL is inline in each handler fn |

## CONVENTIONS

- **Handler pattern**: `async fn(State(state), [auth: AuthUser], [Path(id)], [Json(input)]) -> Result<T, AppError>`
- **JOIN structs are handler-private**: `PostWithUser`, `CommentWithUser` defined in handler files, not model/
- **Validation flow**: `input.validate().map_err(|e| AppError::Validation(e.to_string()))?`
- **Create returns 201**: `Ok((StatusCode::CREATED, Json(response)))`
- **Delete returns 204**: `Ok(StatusCode::NO_CONTENT)`
- **Ownership check**: `if resource.user_id != auth.user_id { return Err(AppError::Forbidden(...)); }` — returns 403 Forbidden
- **Fetch after mutation**: INSERT/UPDATE always followed by SELECT to return fresh data
- **`#[allow(dead_code)]`**: Used on auth.rs public functions — they're used cross-crate but compiler flags them within lib crate

## DATA FLOW

```
Request → Axum Router → [AuthUser extractor] → Handler
  → Validate input (schema)
  → SQL query (inline sqlx)
  → Map to Response type (model)
  → Ok(Json(response)) | Err(AppError)
```

## API ROUTES

| Method | Path | Auth | Handler |
|--------|------|------|---------|
| GET | /health | No | route::health_check |
| POST | /api/auth/register | No | handler::auth::register |
| POST | /api/auth/login | No | handler::auth::login |
| GET | /api/users | Yes | handler::user::list_users |
| GET | /api/users/{id} | Yes | handler::user::get_user |
| GET/PUT/DELETE | /api/users/me | Yes | handler::user::get_me / update_me / delete_me |
| POST | /api/auth/refresh | No | handler::auth::refresh |
| GET/POST | /api/posts | Mixed | handler::post::* (list=public, create=auth) |
| GET/PUT/DELETE | /api/posts/{id} | Mixed | handler::post::* (get=public, update/delete=owner) |
| GET/POST | /api/posts/{post_id}/comments | Mixed | handler::comment::* |
| GET/PUT/DELETE | /api/posts/{post_id}/comments/{comment_id} | Mixed | handler::comment::* |

## NOTES

- Unit tests live inline in `auth.rs` and `error.rs` (`#[cfg(test)]` modules)
- `ensure_post_exists()` in comment handler prevents comments on nonexistent posts (returns 404)
- MySQL error codes mapped: 1062→Conflict, 1451→Conflict, 1452→BadRequest, RowNotFound→NotFound
