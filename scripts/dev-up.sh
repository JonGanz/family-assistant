#!/usr/bin/env bash
# "Repo cloned" -> dependent services running in containers, schema migrated. Safe to re-run;
# destroys nothing. Next: scripts/dev-seed.sh (sample data), scripts/dev-run.sh (app with HMR).
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib-dev.sh

need docker "https://docs.docker.com/engine/install/"
need node "Node.js 24"
need corepack "ships with Node.js"
need cargo "https://rustup.rs"

scripts/dev-key.sh
$COMPOSE up -d --wait postgres ntfy
[ -d apps/web/node_modules ] || (cd apps/web && $PNPM install --frozen-lockfile)
cargo run -q -p fa-server -- migrate
echo "services are up (postgres :55432, ntfy :18090); schema is migrated"
