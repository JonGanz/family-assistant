#!/usr/bin/env bash
# Run the app for development: API + worker (restarted on Rust changes when cargo-watch is
# installed), the offline fixtures, and the Vite dev server with HMR. Open http://localhost:5173.
# Ctrl-C stops everything. Needs scripts/dev-up.sh first.
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib-dev.sh
trap 'kill 0' EXIT

mkdir -p .dev
start_fixtures

if cargo watch --version >/dev/null 2>&1; then
  cargo watch -q -w crates -w bins -w Cargo.toml -x 'run -q -p fa-server -- serve all' 2>&1 | sed -u 's/^/[api] /' &
else
  echo "(cargo-watch not installed: the API will not restart on Rust changes; cargo install cargo-watch)"
  cargo run -q -p fa-server -- serve all 2>&1 | sed -u 's/^/[api] /' &
fi
(cd apps/web && $PNPM run dev --host 127.0.0.1 2>&1 | sed -u 's/^/[web] /') &

wait_api
echo "ready: $WEB_URL (API $API_URL)"
wait
