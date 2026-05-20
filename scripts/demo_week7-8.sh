#!/usr/bin/env bash
# Demo script untuk penilaian Week 7-8.
# Menjalankan setiap langkah verifikasi secara interaktif: tekan Enter
# untuk lanjut, atau "q" untuk berhenti.
#
# Usage:
#   bash scripts/demo_week7-8.sh            # demo lengkap, interaktif
#   bash scripts/demo_week7-8.sh --auto     # demo lengkap tanpa pause
#   bash scripts/demo_week7-8.sh --quick    # versi cepat (test + TLS only)
#
# Prasyarat:
#   - dijalankan dari root blog-api/
#   - docker compose tersedia
#   - .env terisi DATABASE_URL, JWT_SECRET
#   - mkcert atau openssl (untuk gen-certs)
set -euo pipefail

AUTO=false
QUICK=false
for arg in "$@"; do
    case "$arg" in
        --auto)  AUTO=true ;;
        --quick) QUICK=true ;;
        -h|--help)
            sed -n '2,16p' "$0"
            exit 0
            ;;
    esac
done

YELLOW='\033[1;33m'
GREEN='\033[1;32m'
CYAN='\033[1;36m'
RED='\033[1;31m'
NC='\033[0m'

step() {
    local title="$1"
    echo
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
    echo -e "${CYAN}▶ $title${NC}"
    echo -e "${CYAN}━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━${NC}"
}

run() {
    echo -e "${YELLOW}\$ $*${NC}"
    "$@"
}

pause() {
    if $AUTO; then return; fi
    echo
    read -r -p "[Enter] lanjut  |  [q] keluar : " key
    if [[ "$key" == "q" ]]; then
        echo -e "${RED}Demo dihentikan.${NC}"
        exit 0
    fi
}

if [[ ! -f Cargo.toml ]]; then
    echo -e "${RED}Error: jalankan dari root blog-api/ (Cargo.toml tidak ditemukan).${NC}"
    exit 1
fi
if [[ ! -f .env ]]; then
    echo -e "${RED}Error: .env tidak ditemukan. Salin dari .env.example dulu.${NC}"
    exit 1
fi

step "0. Git history Week 7-8 (5 wave + CI fix)"
run git log --oneline -7
pause

step "1. Code quality: cargo fmt"
run cargo fmt --all -- --check
pause

step "2. Code quality: cargo clippy (zero warnings)"
run cargo clippy --all-targets --quiet -- -D warnings
pause

step "3. Start MySQL container"
run docker compose up -d mysql
echo "   Tunggu MySQL ready..."
for i in $(seq 1 20); do
    if docker compose exec -T mysql mysqladmin ping -h localhost --silent 2>/dev/null; then
        echo -e "   ${GREEN}MySQL ready.${NC}"
        break
    fi
    sleep 2
done
pause

step "4. Full test suite — target 68 passed + 2 ignored"
# shellcheck disable=SC2046
DATABASE_URL="${DATABASE_URL:-mysql://blog_user:blog_password@localhost:3306/blog_db}" \
JWT_SECRET="${JWT_SECRET:-dev-secret-change-in-production}" \
RUST_LOG=warn \
RATE_LIMIT_ENABLED=false \
    run cargo test --tests --quiet -- --test-threads=1
pause

if ! $QUICK; then
    step "5. Demo mocking (3 passed)"
    DATABASE_URL="${DATABASE_URL:-mysql://blog_user:blog_password@localhost:3306/blog_db}" \
    JWT_SECRET="${JWT_SECRET:-dev-secret-change-in-production}" \
    RUST_LOG=warn \
    RATE_LIMIT_ENABLED=false \
        run cargo test --test mock_example_test
    pause
fi

step "6. Pastikan TLS cert tersedia"
if [[ -f certs/blog-api.local.pem && -f certs/blog-api.local-key.pem ]]; then
    echo -e "   ${GREEN}Cert sudah ada di certs/.${NC}"
    ls -la certs/
else
    run bash scripts/gen-certs.sh
fi
pause

step "7. Bring up full stack (mysql + api + nginx)"
run docker compose up -d --build
echo "   Tunggu service ready..."
sleep 5
for i in $(seq 1 15); do
    if curl -k -sf https://localhost/health -o /dev/null 2>/dev/null; then
        echo -e "   ${GREEN}Stack ready.${NC}"
        break
    fi
    sleep 3
done
run docker compose ps
pause

step "8. HTTP → HTTPS redirect (301)"
run curl -s -o /dev/null -w "Status: %{http_code}  Location: %{redirect_url}\n" \
    http://localhost/health
pause

step "9. HTTPS + HTTP/2 ke /health"
run curl -k -s -o /dev/null -w "Status: %{http_code}  Protocol: HTTP/%{http_version}\n" \
    https://localhost/health
pause

step "10. Versioned API /api/v1/posts via HTTPS"
run bash -c "curl -k -s https://localhost/api/v1/posts | head -c 400 ; echo"
pause

if ! $QUICK; then
    step "11. TLS handshake (openssl s_client)"
    echo | openssl s_client -connect localhost:443 -servername localhost 2>/dev/null \
        | grep -E "(subject=|issuer=|Protocol|Cipher)" || true
    pause

    step "12. JWT jti claim — login + decode"
    set +e
    TOKEN=$(curl -k -s -X POST https://localhost/api/v1/auth/login \
        -H 'Content-Type: application/json' \
        -d '{"email":"alice@example.com","password":"qwerty"}' \
        | python3 -c "import sys,json; print(json.load(sys.stdin).get('access_token',''))" 2>/dev/null)
    set -e
    if [[ -n "$TOKEN" ]]; then
        echo "Access token (claims decoded):"
        python3 -c "
import json, base64
t = '$TOKEN'
p = t.split('.')[1]
p += '=' * (4 - len(p) % 4)
print(json.dumps(json.loads(base64.urlsafe_b64decode(p)), indent=2))
"
    else
        echo -e "${RED}Login gagal — coba register Alice/Bob dulu atau cek seed migration.${NC}"
    fi
    pause

    step "13. CI workflow inspect"
    run cat .github/workflows/ci.yml
    pause
fi

step "14. Selesai. Stop stack?"
if $AUTO; then
    run docker compose down
else
    read -r -p "Stop docker compose? [Y/n]: " ans
    if [[ "$ans" != "n" && "$ans" != "N" ]]; then
        run docker compose down
    fi
fi

echo -e "${GREEN}✔ Demo selesai.${NC}"
