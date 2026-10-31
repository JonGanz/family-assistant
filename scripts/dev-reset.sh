#!/usr/bin/env bash
# Wipe the dev database volume and .dev/, then bring services up and seed again.
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib-dev.sh

if [ "${FA_DEMO_RESET_YES:-}" != "1" ]; then
  echo "WARNING: this DELETES the dev database volume (deploy/compose.dev.yaml, down -v)."
  echo "         Any household data in the dev DB, plus .dev/ (master key), will be lost."
  printf 'Type "wipe" to continue (anything else aborts): '
  ans=""
  read -r ans < /dev/tty || ans=""
  if [ "$ans" != "wipe" ]; then echo "Aborted."; exit 1; fi
fi

pkill -f 'fa-server serve' || true
pkill -f 'e2e/stubs/server.mjs' || true
pkill -f 'vite' || true
$COMPOSE down -v --remove-orphans
rm -rf .dev

scripts/dev-up.sh
scripts/dev-seed.sh
echo "now run scripts/dev-run.sh"
