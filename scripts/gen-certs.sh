#!/usr/bin/env bash
set -euo pipefail

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
CERT_DIR="$PROJECT_ROOT/certs"
CERT_NAME="blog-api.local"
CERT_FILE="$CERT_DIR/$CERT_NAME.pem"
KEY_FILE="$CERT_DIR/$CERT_NAME-key.pem"

mkdir -p "$CERT_DIR"

if [[ -f "$CERT_FILE" && -f "$KEY_FILE" ]]; then
    echo "[gen-certs] Certificates already exist:"
    echo "  - $CERT_FILE"
    echo "  - $KEY_FILE"
    echo "[gen-certs] Delete them first if you want to regenerate."
    exit 0
fi

if command -v mkcert >/dev/null 2>&1; then
    echo "[gen-certs] mkcert detected; generating locally-trusted certificate ..."
    mkcert -install >/dev/null 2>&1 || true
    mkcert \
        -cert-file "$CERT_FILE" \
        -key-file  "$KEY_FILE" \
        "$CERT_NAME" localhost 127.0.0.1 ::1
    echo "[gen-certs] mkcert certificate created."
else
    echo "[gen-certs] mkcert not found; falling back to openssl self-signed certificate."
    openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
        -keyout "$KEY_FILE" \
        -out    "$CERT_FILE" \
        -subj "/CN=$CERT_NAME" \
        -addext "subjectAltName=DNS:$CERT_NAME,DNS:localhost,IP:127.0.0.1"
    echo "[gen-certs] openssl self-signed certificate created."
    echo "[gen-certs] NOTE: browsers and curl will reject this until you trust it"
    echo "            (use 'curl -k ...' or import the cert into your trust store)."
fi

chmod 600 "$KEY_FILE"
chmod 644 "$CERT_FILE"

echo
echo "[gen-certs] Done."
echo "  Cert: $CERT_FILE"
echo "  Key:  $KEY_FILE"
echo
echo "[gen-certs] To use the '$CERT_NAME' hostname locally, add to /etc/hosts:"
echo "    127.0.0.1  $CERT_NAME"
