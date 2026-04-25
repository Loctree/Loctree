#!/usr/bin/env bash
set -euo pipefail

if [[ $# -lt 1 ]]; then
  echo "Usage: $0 <binary> [<binary>...]"
  exit 1
fi

if [[ "$(uname -s)" != "Darwin" ]]; then
  echo "This smoke check only runs on macOS."
  exit 1
fi

RUN_BINARIES="${RUN_BINARIES:-1}"
status=0

for bin in "$@"; do
  if [[ ! -x "$bin" ]]; then
    echo "[smoke][error] Missing executable: $bin"
    status=1
    continue
  fi

  echo "[smoke] Inspecting $bin"

  unexpected=()
  while IFS= read -r dep; do
    [[ -z "$dep" ]] && continue
    case "$dep" in
      /System/Library/*|/usr/lib/*|@executable_path/*|@loader_path/*)
        ;;
      *)
        unexpected+=("$dep")
        ;;
    esac
  done < <(otool -L "$bin" | awk 'NR > 1 { print $1 }')

  if ((${#unexpected[@]})); then
    echo "[smoke][error] Non-portable runtime dependencies found in $bin:"
    printf '  - %s\n' "${unexpected[@]}"
    status=1
  else
    echo "[smoke] Runtime deps are macOS-system-safe"
  fi

  if [[ "$RUN_BINARIES" == "1" ]]; then
    if "$bin" --version >/dev/null; then
      echo "[smoke] Version check passed"
    else
      echo "[smoke][error] Version check failed for $bin"
      status=1
    fi
  fi
done

exit "$status"
