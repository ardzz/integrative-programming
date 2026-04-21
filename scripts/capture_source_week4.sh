#!/usr/bin/env bash
set -euo pipefail

export PATH="$HOME/go/bin:$PATH"

PROJECT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
EVIDENCE_DIR="${EVIDENCE_DIR:-$PROJECT/evidence/week4}"
FAIL_LOG="${FAIL_LOG:-$EVIDENCE_DIR/capture-failures.log}"
TERMSHOT="${TERMSHOT:-termshot -c --no-shadow -s -m 0 -p 12 -C 120}"

mkdir -p "$EVIDENCE_DIR"

log_failure() {
  local scenario="$1"
  local reason="$2"
  printf '[%s] [source] %s :: %s\n' "$(date -Is)" "$scenario" "$reason" >> "$FAIL_LOG"
}

have_cmd() {
  command -v "$1" >/dev/null 2>&1
}

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

capture_file() {
  local out="$1"
  local file="$2"
  local label="$3"
  if [ ! -f "$file" ]; then
    log_failure "$label" "source file missing: $file"
    return 1
  fi
  if have_cmd bat; then
    capture "$out" "bat --color=always --theme=ansi --paging=never --style=numbers,changes \"$file\"" "$label"
  else
    # Fallback: plain cat with numbered lines (termshot still captures terminal output).
    capture "$out" "cat -n \"$file\"" "$label"
  fi
}

echo "=== Capturing Week 4 Source Screenshots ==="

capture_file "$EVIDENCE_DIR/src-cargo-toml-week4.png"        "$PROJECT/Cargo.toml"                              "cargo-toml"
capture_file "$EVIDENCE_DIR/src-error-week4.png"             "$PROJECT/src/error.rs"                            "error.rs"
capture_file "$EVIDENCE_DIR/src-auth-week4.png"              "$PROJECT/src/auth.rs"                             "auth.rs"
capture_file "$EVIDENCE_DIR/src-route-week4.png"             "$PROJECT/src/route.rs"                            "route.rs"
capture_file "$EVIDENCE_DIR/src-handler-user-week4.png"      "$PROJECT/src/handler/user.rs"                     "handler-user"
capture_file "$EVIDENCE_DIR/src-handler-post-week4.png"      "$PROJECT/src/handler/post.rs"                     "handler-post"
capture_file "$EVIDENCE_DIR/src-handler-comment-week4.png"   "$PROJECT/src/handler/comment.rs"                  "handler-comment"
capture_file "$EVIDENCE_DIR/src-handler-auth-week4.png"      "$PROJECT/src/handler/auth.rs"                     "handler-auth"
capture_file "$EVIDENCE_DIR/src-schema-pagination.png"       "$PROJECT/src/schema/pagination.rs"                "schema-pagination"
capture_file "$EVIDENCE_DIR/src-model-pagination.png"        "$PROJECT/src/model/pagination.rs"                 "model-pagination"
capture_file "$EVIDENCE_DIR/src-schema-user-week4.png"       "$PROJECT/src/schema/user.rs"                      "schema-user"
capture_file "$EVIDENCE_DIR/src-tests-pagination.png"        "$PROJECT/tests/pagination_test.rs"                "tests-pagination"
capture_file "$EVIDENCE_DIR/src-tests-rate-limit.png"        "$PROJECT/tests/rate_limit_test.rs"                "tests-rate-limit"
capture_file "$EVIDENCE_DIR/src-tests-auth-week4.png"        "$PROJECT/tests/auth_test.rs"                      "tests-auth"
capture_file "$EVIDENCE_DIR/src-tests-user-week4.png"        "$PROJECT/tests/user_test.rs"                      "tests-user"
capture_file "$EVIDENCE_DIR/src-docker-compose-week4.png"    "$PROJECT/docker-compose.yml"                      "docker-compose"
capture_file "$EVIDENCE_DIR/src-env-example-week4.png"       "$PROJECT/.env.example"                            "env-example"
capture_file "$EVIDENCE_DIR/src-readme-week4.png"            "$PROJECT/README.md"                               "readme"

echo
echo "=== Source capture complete ==="
ls -la "$EVIDENCE_DIR"/src-*.png 2>/dev/null | awk '{printf "%s\t%s bytes\n", $NF, $5}' || true
if [ -s "$FAIL_LOG" ]; then
  echo
  echo "Some captures failed; see $FAIL_LOG"
fi
