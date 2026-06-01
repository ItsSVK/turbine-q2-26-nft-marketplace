#!/usr/bin/env bash
set -euo pipefail

ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

MPL_CORE_PROGRAM_ID="CoREENxT6tW1HoK8ypY1SxRMZTcVPm7R94rH4PZNhX7d"
MPL_CORE_SO="$ROOT/target/deploy/mpl_core.so"

mkdir -p "$ROOT/target/deploy"

if [[ -f "$MPL_CORE_SO" ]]; then
  echo "setup: mpl_core.so already present, skipping dump"
else
  echo "setup: dumping mpl-core from mainnet → $MPL_CORE_SO"
  solana program dump -u m "$MPL_CORE_PROGRAM_ID" "$MPL_CORE_SO"
fi
