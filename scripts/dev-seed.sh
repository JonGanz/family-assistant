#!/usr/bin/env bash
# Load the fictional example household, a child login, weather, busy sample calendars and a paired
# device into the dev database. Skips when a household already exists; --force runs it anyway (it will fail on a
# household that is already set up, use scripts/dev-reset.sh to start over).
set -euo pipefail
cd "$(dirname "$0")/.."
. scripts/lib-dev.sh
trap 'kill $(jobs -p) 2>/dev/null || true' EXIT

force=0; [ "${1:-}" = "--force" ] && force=1

# Use the running app if there is one; otherwise run one just for the seeding.
if ! api_up; then
  mkdir -p .dev
  start_fixtures
  cargo run -q -p fa-server -- serve all > .dev/seed-serve.log 2>&1 &
  wait_api
fi

if [ "$force" = 0 ] && curl -fsS "$API_URL/api/v1/setup" | grep -q '"completed":true'; then
  echo "already seeded (household exists); use scripts/dev-reset.sh to start over"
  exit 0
fi

export FA_E2E_BASE_URL="$API_URL" FA_E2E_PUBLIC_URL="$WEB_URL"
FA_E2E_USER=alex@example.com FA_E2E_PASSWORD=example-adult-pass1 FA_E2E_PIN=1234 node e2e/seed-household.mjs
echo 'kid1' | cargo run -q -p fa-server -- user add --member riley --username riley@example.com --password-stdin
cargo run -q -p fa-server -- weather fetch
# Busy sample calendars (needs the fixture server and the worker, which run above or in dev-run.sh).
FA_E2E_FIXTURES="http://127.0.0.1:$FIXTURE_PORT" FA_E2E_USER=alex@example.com FA_E2E_PASSWORD=example-adult-pass1 \
  node e2e/seed-dev-calendars.mjs \
  || echo "(sample calendars skipped: is the fixture server up? add them later with: node e2e/seed-dev-calendars.mjs)"
cargo run -q -p fa-server -- device pair --name kitchen

cat <<MSG

Seeded. Log in at $WEB_URL as alex@example.com / example-adult-pass1 (owner PIN 1234)
or riley@example.com / kid1 (child). Sample calendars: Family, Alex, Sam, Riley, Jordan (e2e/fixtures/ics/dev-*.ics)
MSG
