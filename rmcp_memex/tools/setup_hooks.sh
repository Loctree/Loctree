#!/usr/bin/env bash
set -euo pipefail

"$(cd "$(dirname "$0")" && pwd)/install-githooks.sh"

echo "Hooks installed successfully."
