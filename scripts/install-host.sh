#!/usr/bin/env bash
# Register the Lilac native messaging host for Chrome and Firefox on Linux/macOS.
# Usage: ./install-host.sh /path/to/lilac-host [extension_id]
set -euo pipefail

BINARY="${1:-}"
EXT_ID="${2:-}"
if [ -z "$BINARY" ]; then
  echo "Usage: $0 /path/to/lilac-host [chrome_extension_id]" >&2
  exit 1
fi
BINARY="$(cd "$(dirname "$BINARY")" && pwd)/$(basename "$BINARY")"
if [ ! -x "$BINARY" ]; then
  echo "Binary not found or not executable: $BINARY" >&2
  exit 1
fi

MANIFEST_DIR="$HOME/.config/lilac"
mkdir -p "$MANIFEST_DIR"
MANIFEST="$MANIFEST_DIR/lilac_host.json"

ALLOWED="[]"
if [ -n "$EXT_ID" ]; then
  ALLOWED="[\"chrome-extension://${EXT_ID}/\"]"
fi

cat > "$MANIFEST" <<EOF
{
  "name": "lilac_host",
  "description": "Lilac record storage native host",
  "path": "$BINARY",
  "type": "stdio",
  "allowed_origins": $ALLOWED,
  "allowed_extensions": ["lilac@orchid"]
}
EOF
echo "Wrote $MANIFEST"

# Chrome
CHROME_DIR="$HOME/.config/google-chrome/NativeMessagingHosts"
mkdir -p "$CHROME_DIR"
cp "$MANIFEST" "$CHROME_DIR/lilac_host.json"
echo "Registered Chrome."

# Firefox
FF_DIR="$HOME/.mozilla/native-messaging-hosts"
mkdir -p "$FF_DIR"
cp "$MANIFEST" "$FF_DIR/lilac_host.json"
echo "Registered Firefox."

echo "Done. Restart your browser and load the extension from ../extension"