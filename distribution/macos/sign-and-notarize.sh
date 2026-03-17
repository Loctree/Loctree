#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <dist-dir> <output-zip>"
  exit 1
fi

DIST_DIR="$1"
OUTPUT_ZIP="$2"

: "${MACOS_DEVELOPER_ID_APPLICATION:?Set MACOS_DEVELOPER_ID_APPLICATION}"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "Missing dist dir: $DIST_DIR"
  exit 1
fi

# --- Codesign ---
for bin in loctree loct; do
  target="$DIST_DIR/$bin"
  if [[ -f "$target" ]]; then
    codesign --force --timestamp --options runtime --sign "$MACOS_DEVELOPER_ID_APPLICATION" "$target"
  fi
done

rm -f "$OUTPUT_ZIP"
ditto -c -k --keepParent "$DIST_DIR" "$OUTPUT_ZIP"

# --- Notarization (API key auth, 3 retries, 15m timeout) ---

# Resolve auth: prefer API key, fallback to app-specific password
NOTARY_AUTH=()
if [[ -n "${APPLE_API_KEY_BASE64:-}" && -n "${APPLE_API_KEY_ID:-}" && -n "${APPLE_API_ISSUER_ID:-}" ]]; then
  KEY_PATH="$RUNNER_TEMP/AuthKey_${APPLE_API_KEY_ID}.p8"
  echo "$APPLE_API_KEY_BASE64" | base64 --decode > "$KEY_PATH"
  NOTARY_AUTH=(--key "$KEY_PATH" --key-id "$APPLE_API_KEY_ID" --issuer "$APPLE_API_ISSUER_ID")
  echo "Using API key auth (key-id: $APPLE_API_KEY_ID)"
elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_APP_SPECIFIC_PASSWORD:-}" && -n "${APPLE_TEAM_ID:-}" ]]; then
  NOTARY_AUTH=(--apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_APP_SPECIFIC_PASSWORD")
  echo "Using app-specific password auth"
else
  echo "ERROR: No notarization credentials. Set APPLE_API_KEY_* or APPLE_ID+APPLE_APP_SPECIFIC_PASSWORD"
  exit 1
fi

for attempt in 1 2 3; do
  echo "Notarization attempt $attempt/3..."
  if xcrun notarytool submit "$OUTPUT_ZIP" "${NOTARY_AUTH[@]}" --wait --timeout 15m; then
    echo "Notarized archive ready: $OUTPUT_ZIP"
    exit 0
  fi
  echo "Attempt $attempt failed, retrying in 10s..."
  sleep 10
done

echo "Notarization failed after 3 attempts"
exit 1
