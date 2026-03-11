#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 2 ]]; then
  echo "Usage: $0 <dist-dir> <output-zip>"
  exit 1
fi

DIST_DIR="$1"
OUTPUT_ZIP="$2"

: "${MACOS_DEVELOPER_ID_APPLICATION:?Set MACOS_DEVELOPER_ID_APPLICATION}"
: "${APPLE_ID:?Set APPLE_ID}"
: "${APPLE_TEAM_ID:?Set APPLE_TEAM_ID}"
: "${APPLE_APP_SPECIFIC_PASSWORD:?Set APPLE_APP_SPECIFIC_PASSWORD}"

if [[ ! -d "$DIST_DIR" ]]; then
  echo "Missing dist dir: $DIST_DIR"
  exit 1
fi

for bin in loctree loct; do
  target="$DIST_DIR/$bin"
  if [[ -f "$target" ]]; then
    codesign --force --timestamp --options runtime --sign "$MACOS_DEVELOPER_ID_APPLICATION" "$target"
  fi
done

rm -f "$OUTPUT_ZIP"
ditto -c -k --keepParent "$DIST_DIR" "$OUTPUT_ZIP"

xcrun notarytool submit \
  "$OUTPUT_ZIP" \
  --apple-id "$APPLE_ID" \
  --team-id "$APPLE_TEAM_ID" \
  --password "$APPLE_APP_SPECIFIC_PASSWORD" \
  --wait

echo "Notarized archive ready: $OUTPUT_ZIP"
