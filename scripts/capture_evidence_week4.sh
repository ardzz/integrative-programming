#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/go/bin:$PATH"

PROJECT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-$PROJECT/evidence/week4}"
FAIL_LOG="${FAIL_LOG:-$EVIDENCE_DIR/capture-failures.log}"
TERMSHOT="${TERMSHOT:-termshot -c --no-shadow -s -m 0 -p 12 -C 120}"
HOST_PORT="${HOST_PORT:-3030}"
BASE_URL="${BASE_URL:-http://localhost:${HOST_PORT}}"
COMPOSE_OVERRIDE="${COMPOSE_OVERRIDE:-/tmp/blog-api-week4-override.yml}"
COMPOSE_FILES=(-f "$PROJECT/docker-compose.yml" -f "$COMPOSE_OVERRIDE")
KEEP_STACK="${KEEP_STACK:-0}"

mkdir -p "$EVIDENCE_DIR"

log_failure() {
  local scenario="$1"
  local reason="$2"
  printf '[%s] [runtime] %s :: %s\n' "$(date -Is)" "$scenario" "$reason" >> "$FAIL_LOG"
}

have_cmd() { command -v "$1" >/dev/null 2>&1; }

capture() {
  local output_file="$1"
  local command="$2"
  local label="$3"
  if ! have_cmd termshot; then
    log_failure "$label" "termshot not installed"
    return 1
  fi
  local inner
  inner="$TERMSHOT -f $(printf '%q' "$output_file") -- bash -lc $(printf '%q' "$command")"
  if have_cmd script; then
    if script -qc "$inner" /dev/null >/dev/null 2>&1; then
      return 0
    fi
  else
    if eval "$inner" >/dev/null 2>&1; then
      return 0
    fi
  fi
  log_failure "$label" "termshot invocation failed"
  return 1
}

cleanup() {
  if [ "$KEEP_STACK" != "1" ]; then
    docker compose "${COMPOSE_FILES[@]}" down --remove-orphans >/dev/null 2>&1 || true
  fi
  rm -f "$COMPOSE_OVERRIDE"
}

die_runtime() {
  log_failure "stack-bringup" "$1"
  echo "FATAL (runtime): $1" >&2
}

write_override() {
  cat > "$COMPOSE_OVERRIDE" <<YAML
services:
  api:
    ports:
      - "${HOST_PORT}:3000"
    environment:
      RATE_LIMIT_ENABLED: "true"
YAML
}

wait_for_health() {
  local attempt=0
  local max_attempts=60
  while [ $attempt -lt $max_attempts ]; do
    if curl -sf -m 2 "${BASE_URL}/health" >/dev/null 2>&1; then
      return 0
    fi
    attempt=$((attempt + 1))
    sleep 2
  done
  return 1
}

trap cleanup EXIT

echo "=== Bringing up Blog API stack on port ${HOST_PORT} ==="
if ! have_cmd docker; then
  die_runtime "docker not installed"
  exit 0
fi
if ! docker info >/dev/null 2>&1; then
  die_runtime "docker daemon not reachable"
  exit 0
fi

write_override
if ! docker compose "${COMPOSE_FILES[@]}" up -d --build >/dev/null 2>&1; then
  die_runtime "docker compose up failed"
  exit 0
fi

if ! wait_for_health; then
  docker compose "${COMPOSE_FILES[@]}" logs api | tail -40 || true
  die_runtime "health check never returned 200"
  exit 0
fi

echo "API healthy at ${BASE_URL}"

api_json() {
  local method="$1"
  local path="$2"
  local token="${3-}"
  local body="${4-}"
  local args=(-s -X "$method" "${BASE_URL}${path}" -H 'Content-Type: application/json')
  if [ -n "$token" ]; then args+=(-H "Authorization: Bearer ${token}"); fi
  if [ -n "$body" ]; then args+=(-d "$body"); fi
  curl "${args[@]}"
}

extract() {
  jq -r "$1" 2>/dev/null || true
}

echo "=== Seeding test data ==="

ALICE_LOGIN=$(api_json POST /api/auth/login "" '{"email":"alice@example.com","password":"qwerty"}')
ALICE_ACCESS=$(printf '%s' "$ALICE_LOGIN" | extract '.access_token // .token // empty')
ALICE_REFRESH=$(printf '%s' "$ALICE_LOGIN" | extract '.refresh_token // empty')

BOB_LOGIN=$(api_json POST /api/auth/login "" '{"email":"bob@example.com","password":"qwerty"}')
BOB_ACCESS=$(printf '%s' "$BOB_LOGIN" | extract '.access_token // .token // empty')

CAROL_EMAIL="carol-week4-$(date +%s)@example.com"
CAROL_REGISTER=$(api_json POST /api/auth/register "" "{\"name\":\"Carol\",\"email\":\"${CAROL_EMAIL}\",\"password\":\"qwerty\"}")
CAROL_ACCESS=$(printf '%s' "$CAROL_REGISTER" | extract '.access_token // .token // empty')

if [ -z "$ALICE_ACCESS" ] || [ -z "$BOB_ACCESS" ] || [ -z "$CAROL_ACCESS" ]; then
  log_failure "seed-auth" "missing tokens (alice=${#ALICE_ACCESS} bob=${#BOB_ACCESS} carol=${#CAROL_ACCESS})"
fi

BOB_POST=$(api_json POST /api/posts "$BOB_ACCESS" '{"title":"Bobs Week4 Post","content":"Owned by Bob for ownership tests.","status":"published"}')
BOB_POST_ID=$(printf '%s' "$BOB_POST" | extract '.id // .data.id // empty')
ALICE_POST=$(api_json POST /api/posts "$ALICE_ACCESS" '{"title":"Alices Week4 Post","content":"Owned by Alice; Bob will try to delete a comment here.","status":"published"}')
ALICE_POST_ID=$(printf '%s' "$ALICE_POST" | extract '.id // .data.id // empty')

COMMENT_ON_ALICE=$(api_json POST "/api/posts/${ALICE_POST_ID}/comments" "$ALICE_ACCESS" '{"comment":"Alice own comment; Bob will try to delete this."}')
COMMENT_ON_ALICE_ID=$(printf '%s' "$COMMENT_ON_ALICE" | extract '.id // .data.id // empty')

echo "Seed IDs :: BOB_POST=${BOB_POST_ID} ALICE_POST=${ALICE_POST_ID} COMMENT=${COMMENT_ON_ALICE_ID}"

capture "$EVIDENCE_DIR/01-pagination-users.png" \
  "echo '=== GET /api/users?page=1&per_page=5 ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -H 'Authorization: Bearer ${ALICE_ACCESS}' '${BASE_URL}/api/users?page=1&per_page=5' | jq ." \
  "01-pagination-users"

capture "$EVIDENCE_DIR/02-pagination-posts.png" \
  "echo '=== GET /api/posts?page=1&per_page=3 ===' && curl -s -w '\nHTTP Status: %{http_code}\n' '${BASE_URL}/api/posts?page=1&per_page=3' | jq ." \
  "02-pagination-posts"

capture "$EVIDENCE_DIR/03-pagination-comments.png" \
  "echo '=== GET /api/posts/${ALICE_POST_ID}/comments?page=1&per_page=2 ===' && curl -s -w '\nHTTP Status: %{http_code}\n' '${BASE_URL}/api/posts/${ALICE_POST_ID}/comments?page=1&per_page=2' | jq ." \
  "03-pagination-comments"

capture "$EVIDENCE_DIR/04-pagination-invalid.png" \
  "echo '=== GET /api/posts?per_page=150 (expect 400) ===' && curl -si '${BASE_URL}/api/posts?per_page=150'" \
  "04-pagination-invalid"

capture "$EVIDENCE_DIR/05-me-get.png" \
  "echo '=== GET /api/users/me ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -H 'Authorization: Bearer ${CAROL_ACCESS}' '${BASE_URL}/api/users/me' | jq ." \
  "05-me-get"

capture "$EVIDENCE_DIR/06-me-update.png" \
  "echo '=== PUT /api/users/me ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -X PUT -H 'Content-Type: application/json' -H 'Authorization: Bearer ${CAROL_ACCESS}' -d '{\"name\":\"Carol Renamed\"}' '${BASE_URL}/api/users/me' | jq ." \
  "06-me-update"

capture "$EVIDENCE_DIR/07-me-delete.png" \
  "echo '=== DELETE /api/users/me (expect 204) ===' && curl -si -X DELETE -H 'Authorization: Bearer ${CAROL_ACCESS}' '${BASE_URL}/api/users/me'" \
  "07-me-delete"

# Scenario 08 — Alice updates Bob's post (expect 403)
capture "$EVIDENCE_DIR/08-403-update-other-post.png" \
  "echo '=== PUT /api/posts/${BOB_POST_ID} as Alice (expect 403) ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -X PUT -H 'Content-Type: application/json' -H 'Authorization: Bearer ${ALICE_ACCESS}' -d '{\"title\":\"Hacked\",\"status\":\"published\"}' '${BASE_URL}/api/posts/${BOB_POST_ID}' | jq ." \
  "08-403-update-other-post"

# Scenario 09 — Bob deletes Alice's comment (expect 403)
capture "$EVIDENCE_DIR/09-403-delete-other-comment.png" \
  "echo '=== DELETE /api/posts/${ALICE_POST_ID}/comments/${COMMENT_ON_ALICE_ID} as Bob (expect 403) ===' && curl -si -X DELETE -H 'Authorization: Bearer ${BOB_ACCESS}' '${BASE_URL}/api/posts/${ALICE_POST_ID}/comments/${COMMENT_ON_ALICE_ID}'" \
  "09-403-delete-other-comment"

# Scenario 10 — PUT /api/users/1 not allowed (expect 405)
capture "$EVIDENCE_DIR/10-put-users-id-not-allowed.png" \
  "echo '=== PUT /api/users/1 (expect 405/404) ===' && curl -si -X PUT -H 'Content-Type: application/json' -H 'Authorization: Bearer ${ALICE_ACCESS}' -d '{\"name\":\"Nope\"}' '${BASE_URL}/api/users/1'" \
  "10-put-users-id-not-allowed"

sleep 1

# Scenario 11 — auth rate limit 429 (6 rapid failed logins)
capture "$EVIDENCE_DIR/11-rate-limit-auth-429.png" \
  "echo '=== 6 rapid failed logins (expect 429 on last) ===' && for i in 1 2 3 4 5 6; do printf '[%s] ' \$i; curl -s -o /dev/null -w 'HTTP %{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{\"email\":\"alice@example.com\",\"password\":\"WRONG\"}' '${BASE_URL}/api/auth/login'; done" \
  "11-rate-limit-auth-429"

sleep 1

# Scenario 12 — global rate limit 429 via 70 rapid GET /api/posts (global bucket is 60 burst).
capture "$EVIDENCE_DIR/12-rate-limit-global-429.png" \
  "echo '=== 70 rapid GET /api/posts (expect tail 429) ===' && for i in \$(seq 1 70); do printf '[%02d] ' \$i; curl -s -o /dev/null -w 'HTTP %{http_code}\n' '${BASE_URL}/api/posts'; done | tail -20" \
  "12-rate-limit-global-429"

sleep 2

capture "$EVIDENCE_DIR/13-refresh-login-returns-both-tokens.png" \
  "echo '=== POST /api/auth/login (shows access + refresh) ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{\"email\":\"alice@example.com\",\"password\":\"qwerty\"}' '${BASE_URL}/api/auth/login' | jq ." \
  "13-refresh-login-returns-both-tokens"

# Refresh for scenario 14 — need a fresh refresh token because ALICE_REFRESH may be stale after bucket waits
FRESH_LOGIN=$(api_json POST /api/auth/login "" '{"email":"alice@example.com","password":"qwerty"}')
FRESH_REFRESH=$(printf '%s' "$FRESH_LOGIN" | extract '.refresh_token // empty')
FRESH_ACCESS=$(printf '%s' "$FRESH_LOGIN" | extract '.access_token // empty')

capture "$EVIDENCE_DIR/14-refresh-flow-success.png" \
  "echo '=== POST /api/auth/refresh (expect new token pair) ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{\"refresh_token\":\"${FRESH_REFRESH}\"}' '${BASE_URL}/api/auth/refresh' | jq ." \
  "14-refresh-flow-success"

capture "$EVIDENCE_DIR/15-refresh-reject-access-token.png" \
  "echo '=== POST /api/auth/refresh with ACCESS token (expect 401) ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -X POST -H 'Content-Type: application/json' -d '{\"refresh_token\":\"${FRESH_ACCESS}\"}' '${BASE_URL}/api/auth/refresh' | jq ." \
  "15-refresh-reject-access-token"

capture "$EVIDENCE_DIR/16-refresh-reject-refresh-on-protected.png" \
  "echo '=== GET /api/users/me with REFRESH token (expect 401) ===' && curl -s -w '\nHTTP Status: %{http_code}\n' -H 'Authorization: Bearer ${FRESH_REFRESH}' '${BASE_URL}/api/users/me' | jq ." \
  "16-refresh-reject-refresh-on-protected"

# Scenario 17 — cargo test. Skip if cargo or MySQL not available from host; still try.
capture "$EVIDENCE_DIR/17-cargo-test-week4.png" \
  "echo '=== cargo test (tail) ===' && cd '$PROJECT' && (DATABASE_URL=\"mysql://blog_user:blog_password@127.0.0.1:3306/blog_db\" cargo test --quiet 2>&1 | tail -25) || true" \
  "17-cargo-test-week4"

capture "$EVIDENCE_DIR/18-cargo-clippy-week4.png" \
  "echo '=== cargo clippy -- -D warnings (tail) ===' && cd '$PROJECT' && (cargo clippy --quiet -- -D warnings 2>&1 | tail -20; echo '---exit='\$?) || true" \
  "18-cargo-clippy-week4"

capture "$EVIDENCE_DIR/19-docker-compose-up.png" \
  "echo '=== docker compose ps ===' && docker compose -f '$PROJECT/docker-compose.yml' -f '$COMPOSE_OVERRIDE' ps" \
  "19-docker-compose-up"

capture "$EVIDENCE_DIR/20-gap-analysis-command.png" \
  "echo '=== Gap analysis: health + readiness line ===' && curl -s '${BASE_URL}/health' && echo && echo 'Week 4 API ready'" \
  "20-gap-analysis-command"

echo
echo "=== Runtime capture complete ==="
ls -la "$EVIDENCE_DIR"/[0-9]*.png 2>/dev/null | awk '{printf "%s\t%s bytes\n", $NF, $5}' || true
if [ -s "$FAIL_LOG" ]; then
  echo
  echo "Some captures failed; see $FAIL_LOG"
fi
