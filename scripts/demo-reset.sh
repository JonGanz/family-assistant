#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
trap 'kill 0' EXIT   # stop the backgrounded fixture server with the script

# --- Safety prompt: this deletes the dev database volume ---
if [ "${FA_DEMO_RESET_YES:-}" != "1" ]; then
  echo "WARNING: this DELETES the dev database volume (deploy/compose.dev.yaml, down -v)."
  echo "         Any household data in the dev DB, plus .dev/ (master key), will be lost."
  printf 'Type "wipe" to continue (anything else aborts): '
  ans=""
  read -r ans < /dev/tty || ans=""
  if [ "$ans" != "wipe" ]; then echo "Aborted."; exit 1; fi
fi

# --- 1. Tear down prior setup ---
pkill -f 'fa-server serve' || true                     # stop any running api/worker
pkill -f 'e2e/stubs/server.mjs' || true                # stop any old fixture server
docker compose -f deploy/compose.dev.yaml down -v --remove-orphans   # dev Postgres + ntfy, incl. data volume
rm -rf .dev apps/web/dist                              # old master key, old web build

# --- 2. Build what is local ---
(cd apps/web && corepack pnpm@9 install --frozen-lockfile && corepack pnpm@9 run build)
cargo build -p fa-server

# --- 3. Fresh database, key and example data ---
docker compose -f deploy/compose.dev.yaml up -d --wait postgres ntfy
scripts/dev-key.sh
cargo run -p fa-server -- migrate
cargo run -p fa-server -- config import config/examples/household.example.toml

# Example logins (local-only passwords; the CLI reads one line from stdin when piped)
echo 'example-adult-pass1' | cargo run -p fa-server -- user add --member alex  --username alex@example.com --password-stdin
echo 'kid1'                | cargo run -p fa-server -- user add --member riley --username riley@example.com --password-stdin
echo '1234'                | cargo run -p fa-server -- pin set --member alex --pin-stdin

# --- 4. Offline fixtures: fake weather (freezing + rain) and sample ICS calendars on :18091 ---
node e2e/stubs/server.mjs 18091 > /dev/null 2>&1 &
export FA_OPEN_METEO_URL=http://127.0.0.1:18091
cargo run -p fa-server -- weather fetch

# --- 5. Pair a dashboard device (prints its URL) ---
cargo run -p fa-server -- device pair --name kitchen

# --- 6. Run API + worker in the foreground (Ctrl-C to stop) ---
cargo run -p fa-server -- serve all
