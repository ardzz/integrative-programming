#!/usr/bin/env bash
set -euo pipefail

BASE_URL="${BASE_URL:-http://localhost:3000}"
OUTPUT_DIR="${OUTPUT_DIR:-evidence/week4/demo}"
RATE_LIMIT_COOLDOWN_SECONDS="${RATE_LIMIT_COOLDOWN_SECONDS:-65}"

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT_DIR="$(cd "${SCRIPT_DIR}/.." && pwd)"

if [[ "$OUTPUT_DIR" != /* ]]; then
  OUTPUT_DIR="${PROJECT_DIR}/${OUTPUT_DIR}"
fi

mkdir -p "$OUTPUT_DIR"

curl -fsS "$BASE_URL/health" >/dev/null || { echo "Server not reachable at $BASE_URL"; exit 1; }

require_cmd() {
  command -v "$1" >/dev/null 2>&1 || {
    echo "Missing required command: $1" >&2
    exit 1
  }
}

require_cmd curl
require_cmd jq

unique_suffix() {
  if command -v uuidgen >/dev/null 2>&1; then
    uuidgen | cut -c1-8
  else
    date +%s
  fi
}

extract_body() {
  awk '
    BEGIN { body = 0 }
    /^\r?$/ {
      if (body == 0) {
        body = 1
        next
      }
    }
    body {
      sub(/\r$/, "")
      if ($0 ~ /^=== HTTP [0-9]{3} in /) {
        next
      }
      print
    }
  ' <<<"$1"
}

extract_status() {
  grep -oE '=== HTTP [0-9]{3} in [0-9.]+s ===' <<<"$1" | tail -1 | grep -oE '[0-9]{3}' | head -1
}

json_field() {
  local raw="$1"
  local query="$2"
  extract_body "$raw" | jq -r "$query"
}

print_divider() {
  printf '\n===== %s =====\n' "$1"
}

run_curl() {
  local step="$1"
  local method="$2"
  local path="$3"
  local token="${4:-}"
  local body="${5:-}"
  local outfile="$OUTPUT_DIR/${step}.txt"
  local -a args=(curl -si -X "$method" "${BASE_URL}${path}" -H 'Content-Type: application/json' -w "\n=== HTTP %{http_code} in %{time_total}s ===\n")

  if [[ -n "$token" ]]; then
    args+=(-H "Authorization: Bearer ${token}")
  fi

  if [[ -n "$body" ]]; then
    args+=(-d "$body")
  fi

  printf '$ %s %s\n' "$method" "$path"
  local raw
  raw="$("${args[@]}" | tee "$outfile")"
  printf '\n%s\n\n' "$raw"
  LAST_STATUS="$(extract_status "$raw")"
  LAST_BODY="$(extract_body "$raw")"
  LAST_OUTFILE="$outfile"
}

run_rate_limit_burst() {
  local step="$1"
  local outfile="$OUTPUT_DIR/${step}.txt"
  : > "$outfile"

  printf '$ POST /api/auth/login (x7 rapid invalid attempts)\n'

  local raw_all=""
  local idx
  for idx in 1 2 3 4 5 6 7; do
    local raw
    raw="$(curl -si -X POST "${BASE_URL}/api/auth/login" -H 'Content-Type: application/json' -d '{"email":"nobody@example.com","password":"wrong-password"}' -w "\n=== HTTP %{http_code} in %{time_total}s ===\n")"
    {
      printf -- '--- Attempt %s ---\n' "$idx"
      printf '%s\n\n' "$raw"
    } | tee -a "$outfile"
    raw_all+="--- Attempt ${idx} ---\n${raw}\n\n"
  done

  printf '\n'
  LAST_RATE_LIMIT_OUTFILE="$outfile"
  LAST_RATE_LIMIT_RAW="$raw_all"
}

SCENARIO_PAGINATION=FAIL
SCENARIO_FORBIDDEN=FAIL
SCENARIO_ME=FAIL
SCENARIO_RATE_LIMIT=FAIL
SCENARIO_REFRESH=FAIL

suffix="$(unique_suffix)"
USER_A_EMAIL="week4-a-${suffix}@example.com"
USER_B_EMAIL="week4-b-${suffix}@example.com"
USER_C_EMAIL="week4-c-${suffix}@example.com"
PASSWORD="qwerty"

print_divider 'Register User A'
run_curl "01-register-user-a" POST /api/auth/register "" "{\"name\":\"Week4 User A\",\"email\":\"${USER_A_EMAIL}\",\"password\":\"${PASSWORD}\"}"
USER_A_REGISTER_RAW="$LAST_BODY"
USER_A_ACCESS="$(jq -r '.access_token // empty' <<<"$USER_A_REGISTER_RAW")"
USER_A_REFRESH="$(jq -r '.refresh_token // empty' <<<"$USER_A_REGISTER_RAW")"
USER_A_ID="$(jq -r '.user.id // empty' <<<"$USER_A_REGISTER_RAW")"

print_divider 'Register User B'
run_curl "02-register-user-b" POST /api/auth/register "" "{\"name\":\"Week4 User B\",\"email\":\"${USER_B_EMAIL}\",\"password\":\"${PASSWORD}\"}"
USER_B_REGISTER_RAW="$LAST_BODY"
USER_B_ACCESS="$(jq -r '.access_token // empty' <<<"$USER_B_REGISTER_RAW")"

print_divider 'Register User C'
run_curl "03-register-user-c" POST /api/auth/register "" "{\"name\":\"Week4 User C\",\"email\":\"${USER_C_EMAIL}\",\"password\":\"${PASSWORD}\"}"
USER_C_REGISTER_RAW="$LAST_BODY"
USER_C_ACCESS="$(jq -r '.access_token // empty' <<<"$USER_C_REGISTER_RAW")"

print_divider '1. Pagination Demo'
run_curl "04-pagination-posts-valid" GET '/api/posts?page=1&per_page=2' "$USER_A_ACCESS"
PAGINATION_VALID_STATUS="$LAST_STATUS"
PAGINATION_VALID_HAS_DATA="$(jq -r 'has("data") and has("meta")' <<<"$LAST_BODY")"
run_curl "05-pagination-posts-invalid" GET '/api/posts?per_page=200' "$USER_A_ACCESS"
PAGINATION_INVALID_STATUS="$LAST_STATUS"
if [[ "$PAGINATION_VALID_STATUS" == "200" && "$PAGINATION_VALID_HAS_DATA" == "true" && "$PAGINATION_INVALID_STATUS" == "400" ]]; then
  SCENARIO_PAGINATION=PASS
fi

print_divider '2. 403 Forbidden Demo'
run_curl "06-create-post-user-a" POST /api/posts "$USER_A_ACCESS" '{"title":"Week 4 Security Demo Post","content":"Owned by User A","status":"published"}'
POST_ID="$(jq -r '.id // empty' <<<"$LAST_BODY")"
run_curl "07-update-post-user-b-forbidden" PUT "/api/posts/${POST_ID}" "$USER_B_ACCESS" '{"title":"Hijacked","content":"Hijacked","status":"draft"}'
FORBIDDEN_UPDATE_STATUS="$LAST_STATUS"
run_curl "08-delete-post-user-b-forbidden" DELETE "/api/posts/${POST_ID}" "$USER_B_ACCESS"
FORBIDDEN_DELETE_STATUS="$LAST_STATUS"
if [[ -n "$POST_ID" && "$FORBIDDEN_UPDATE_STATUS" == "403" && "$FORBIDDEN_DELETE_STATUS" == "403" ]]; then
  SCENARIO_FORBIDDEN=PASS
fi

print_divider '3. /me Endpoint Demo'
run_curl "09-users-me-get" GET /api/users/me "$USER_A_ACCESS"
ME_GET_STATUS="$LAST_STATUS"
ME_EMAIL="$(jq -r '.email // empty' <<<"$LAST_BODY")"
ME_ID="$(jq -r '.id // empty' <<<"$LAST_BODY")"
run_curl "10-users-me-put" PUT /api/users/me "$USER_A_ACCESS" '{"name":"Week4 User A Updated"}'
ME_PUT_STATUS="$LAST_STATUS"
ME_UPDATED_NAME="$(jq -r '.name // empty' <<<"$LAST_BODY")"
run_curl "11-users-id-put-removed" PUT /api/users/1 "$USER_A_ACCESS" '{"name":"Should Fail"}'
ME_LEGACY_STATUS="$LAST_STATUS"
if [[ "$ME_GET_STATUS" == "200" && "$ME_PUT_STATUS" == "200" && "$ME_EMAIL" == "$USER_A_EMAIL" && "$ME_ID" == "$USER_A_ID" && "$ME_UPDATED_NAME" == 'Week4 User A Updated' && ( "$ME_LEGACY_STATUS" == "404" || "$ME_LEGACY_STATUS" == "405" ) ]]; then
  SCENARIO_ME=PASS
fi

print_divider '4. Rate Limiting Demo'
run_rate_limit_burst "12-rate-limit-login-burst"
RATE_LIMIT_429_COUNT="$(grep -c '=== HTTP 429 in' <<<"$LAST_RATE_LIMIT_RAW" || true)"
if [[ "$RATE_LIMIT_429_COUNT" -ge 1 ]]; then
  SCENARIO_RATE_LIMIT=PASS
fi

printf 'Waiting %ss for auth rate-limit window to cool down before refresh flow.\n' "$RATE_LIMIT_COOLDOWN_SECONDS"
sleep "$RATE_LIMIT_COOLDOWN_SECONDS"

print_divider '5. Refresh Token Flow Demo'
run_curl "13-login-user-a" POST /api/auth/login "" "{\"email\":\"${USER_A_EMAIL}\",\"password\":\"${PASSWORD}\"}"
LOGIN_STATUS="$LAST_STATUS"
LOGIN_ACCESS="$(jq -r '.access_token // empty' <<<"$LAST_BODY")"
LOGIN_REFRESH="$(jq -r '.refresh_token // empty' <<<"$LAST_BODY")"
run_curl "14-refresh-valid" POST /api/auth/refresh "" "{\"refresh_token\":\"${LOGIN_REFRESH}\"}"
REFRESH_OK_STATUS="$LAST_STATUS"
NEW_ACCESS="$(jq -r '.access_token // empty' <<<"$LAST_BODY")"
NEW_REFRESH="$(jq -r '.refresh_token // empty' <<<"$LAST_BODY")"
run_curl "15-refresh-with-access-token" POST /api/auth/refresh "" "{\"refresh_token\":\"${LOGIN_ACCESS}\"}"
REFRESH_ACCESS_STATUS="$LAST_STATUS"
run_curl "16-users-me-with-refresh-token" GET /api/users/me "$LOGIN_REFRESH"
REFRESH_AS_BEARER_STATUS="$LAST_STATUS"
if [[ "$LOGIN_STATUS" == "200" && "$REFRESH_OK_STATUS" == "200" && -n "$NEW_ACCESS" && -n "$NEW_REFRESH" && "$NEW_ACCESS" != "$LOGIN_ACCESS" && "$NEW_REFRESH" != "$LOGIN_REFRESH" && "$REFRESH_ACCESS_STATUS" == "401" && "$REFRESH_AS_BEARER_STATUS" == "401" ]]; then
  SCENARIO_REFRESH=PASS
fi

printf '\n=== DEMO SUMMARY ===\n'
printf '1) pagination=%s\n' "$SCENARIO_PAGINATION"
printf '2) forbidden=%s\n' "$SCENARIO_FORBIDDEN"
printf '3) me=%s\n' "$SCENARIO_ME"
printf '4) rate_limit=%s\n' "$SCENARIO_RATE_LIMIT"
printf '5) refresh=%s\n' "$SCENARIO_REFRESH"

if [[ "$SCENARIO_PAGINATION" == PASS && "$SCENARIO_FORBIDDEN" == PASS && "$SCENARIO_ME" == PASS && "$SCENARIO_RATE_LIMIT" == PASS && "$SCENARIO_REFRESH" == PASS ]]; then
  exit 0
fi

exit 1
