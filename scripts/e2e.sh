#!/usr/bin/env bash
# End-to-end tests against a throwaway stack: Postgres, `serve api` and `serve worker` as
# separate processes, an ICS + Open-Meteo fixture server (Node), Radicale (CalDAV), ntfy
# (notifications), GreenMail (IMAP + SMTP for the mail specs), then the
# Playwright API specs (e2e/) and UI specs (apps/web/e2e/).
# Env: FA_E2E_PG_PORT (55499), FA_E2E_PORT (18080), FA_E2E_STUB_PORT (18091),
#      FA_E2E_RADICALE_PORT (18092), FA_E2E_NTFY_PORT (18093),
#      FA_E2E_IMAP_PORT (18143), FA_E2E_SMTP_PORT (18025), FA_E2E_LAN_PORT (18081),
#      FA_E2E_SETUP_PORT (18094, empty-database instance for the API setup spec),
#      FA_E2E_WIZARD_PORT (18095, empty-database instance for the UI wizard, bound on all
#      interfaces and browsed through the host's LAN address: a plain-HTTP, non-secure context),
#      FA_E2E_SKIP_BUILD=1 to reuse apps/web/dist,
#      FA_E2E_HOLD=<seconds> to keep the seeded stack up for that long before the specs (debugging).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD

PG_PORT=${FA_E2E_PG_PORT:-55499}
PORT=${FA_E2E_PORT:-18080}
STUB_PORT=${FA_E2E_STUB_PORT:-18091}
RAD_PORT=${FA_E2E_RADICALE_PORT:-18092}
NTFY_PORT=${FA_E2E_NTFY_PORT:-18093}
IMAP_PORT=${FA_E2E_IMAP_PORT:-18143}
SMTP_PORT=${FA_E2E_SMTP_PORT:-18025}
LAN_PORT=${FA_E2E_LAN_PORT:-18081}
SETUP_PORT=${FA_E2E_SETUP_PORT:-18094}
WIZARD_PORT=${FA_E2E_WIZARD_PORT:-18095}
PROJECT=fa-e2e-$$
WORK=$(mktemp -d)
PIDS=()

cleanup() {
  for p in "${PIDS[@]:-}"; do [[ -n $p ]] && kill "$p" 2>/dev/null || true; done
  docker rm -f -v "$PROJECT-pg" "$PROJECT-radicale" "$PROJECT-ntfy" "$PROJECT-greenmail" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

dump_logs() {
  for f in api worker stub setup wizard; do echo "---- $f log ----"; tail -n 40 "$WORK/$f.log" 2>/dev/null || true; done
}

export SQLX_OFFLINE=true # compile-time queries use the committed .sqlx data, not the not-yet-started stack
export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PG_PORT/fa"
export FA_BIND_ADDR="127.0.0.1:$PORT"
export FA_PUBLIC_BASE_URL="http://127.0.0.1:$PORT"
export FA_COOKIE_SECURE=false
export FA_LOG_LEVEL=info
export FA_LOG_FORMAT=pretty
export FA_MASTER_KEY_PATH="$WORK/master.key"
export FA_WEB_DIST_DIR="$ROOT/apps/web/dist"
export FA_OPEN_METEO_URL="http://127.0.0.1:$STUB_PORT"
export FA_ALLOW_HTTP_FEEDS=true
export FA_ALLOW_PRIVATE_HOSTS=true # the fixture server, Radicale, GreenMail and the stubs are all on 127.0.0.1
export FA_NTFY_URL="http://127.0.0.1:$NTFY_PORT"
export FA_ALLOW_PLAIN_IMAP=true # test-only: GreenMail speaks plain IMAP
# The home-network dashboard listener (D-064); loopback is in its default allowed ranges.
export FA_LAN_DASHBOARD_URL="http://127.0.0.1:$LAN_PORT"
export FA_LAN_DASHBOARD_BIND="127.0.0.1:$LAN_PORT"
head -c 32 /dev/urandom > "$FA_MASTER_KEY_PATH"
chmod 400 "$FA_MASTER_KEY_PATH"

echo "== build"
cargo build -q -p fa-server
if [[ -z ${FA_E2E_SKIP_BUILD:-} || ! -f $FA_WEB_DIST_DIR/index.html ]]; then
  (cd apps/web && corepack pnpm@9 install --frozen-lockfile >/dev/null && corepack pnpm@9 run build >/dev/null)
fi
BIN=$ROOT/target/debug/fa-server

echo "== postgres on :$PG_PORT, greenmail on :$IMAP_PORT/:$SMTP_PORT, radicale on :$RAD_PORT, ntfy on :$NTFY_PORT, fixture server on :$STUB_PORT"
docker run -d --name "$PROJECT-pg" -e POSTGRES_USER=fa -e POSTGRES_PASSWORD=fa -e POSTGRES_DB=fa \
  -p "127.0.0.1:$PG_PORT:5432" pgvector/pgvector:pg16 >/dev/null
docker run -d --name "$PROJECT-radicale" -p "127.0.0.1:$RAD_PORT:5232" \
  tomsquest/docker-radicale:3.8.3.0 >/dev/null
# Lift ntfy's per-IP request limit (burst 60): the specs poll the topics from the same address
# that fa-server publishes from, and a 429 would delay notifications.
docker run -d --name "$PROJECT-ntfy" -p "127.0.0.1:$NTFY_PORT:80" \
  -e NTFY_VISITOR_REQUEST_LIMIT_BURST=100000 -e NTFY_VISITOR_REQUEST_LIMIT_REPLENISH=10ms \
  binwiederhier/ntfy:v2.17.0 serve >/dev/null
docker run -d --name "$PROJECT-greenmail" -p "127.0.0.1:$IMAP_PORT:3143" -p "127.0.0.1:$SMTP_PORT:3025" \
  -e GREENMAIL_OPTS='-Dgreenmail.setup.test.imap -Dgreenmail.setup.test.smtp -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled' \
  greenmail/standalone:2.1.8 >/dev/null
node e2e/stubs/server.mjs "$STUB_PORT" >"$WORK/stub.log" 2>&1 &
PIDS+=($!)
for _ in $(seq 60); do
  docker exec "$PROJECT-pg" pg_isready -U fa -d fa >/dev/null 2>&1 \
    && curl -fsS "http://127.0.0.1:$RAD_PORT/" >/dev/null 2>&1 \
    && curl -fsS "http://127.0.0.1:$NTFY_PORT/v1/health" >/dev/null 2>&1 \
    && (exec 3<>"/dev/tcp/127.0.0.1/$IMAP_PORT" && read -r -t 2 g <&3 && [[ $g == "* OK"* ]]) 2>/dev/null \
    && (exec 3<>"/dev/tcp/127.0.0.1/$SMTP_PORT" && read -r -t 2 g <&3 && [[ $g == 220* ]]) 2>/dev/null \
    && curl -fsS "http://127.0.0.1:$STUB_PORT/ics/school.ics" >/dev/null 2>&1 && break
  sleep 1
done
sleep 2 # the postgres entrypoint restarts the server once after init

echo "== seed radicale"
RAD=http://127.0.0.1:$RAD_PORT
curl -fsS -o /dev/null -u e2e:e2e -X MKCOL -H 'Content-Type: application/xml' \
  --data '<?xml version="1.0"?><mkcol xmlns="DAV:" xmlns:C="urn:ietf:params:xml:ns:caldav"><set><prop><resourcetype><collection/><C:calendar/></resourcetype><displayname>Family</displayname></prop></set></mkcol>' \
  "$RAD/e2e/family/"
day() { date -u -d "+$1 days" +%Y%m%d; }
put_event() { # uid, offset-days, summary
  printf 'BEGIN:VCALENDAR\r\nVERSION:2.0\r\nPRODID:-//e2e//EN\r\nBEGIN:VEVENT\r\nUID:%s@family.example.com\r\nDTSTAMP:20260101T000000Z\r\nDTSTART:%sT190000Z\r\nDTEND:%sT203000Z\r\nSUMMARY:%s\r\nEND:VEVENT\r\nEND:VCALENDAR\r\n' \
    "$1" "$(day "$2")" "$(day "$2")" "$3" |
    curl -fsS -o /dev/null -u e2e:e2e -X PUT -H 'Content-Type: text/calendar' --data-binary @- "$RAD/e2e/family/$1.ics"
}
put_event bookclub 4 "Book Club"
put_event pottery 6 "Pottery Night"
put_event vet 9 "Vet Visit"

echo "== seed household"
"$BIN" migrate
"$BIN" config import config/examples/household.example.toml
export FA_E2E_USER=alex@example.com FA_E2E_PASSWORD=correct-horse-battery-staple
export FA_E2E_CHILD_USER=riley@example.com FA_E2E_CHILD_PASSWORD=dino1234
export FA_E2E_SAM_USER=sam@example.com FA_E2E_SAM_PASSWORD=sam-and-alex-share-chores
printf '%s\n' "$FA_E2E_PASSWORD" | "$BIN" user add --member alex --username "$FA_E2E_USER" --password-stdin
printf '%s\n' "$FA_E2E_SAM_PASSWORD" | "$BIN" user add --member sam --username "$FA_E2E_SAM_USER" --password-stdin
printf '%s\n' "$FA_E2E_CHILD_PASSWORD" | "$BIN" user add --member riley --username "$FA_E2E_CHILD_USER" --password-stdin
"$BIN" weather fetch

echo "== empty-database instances for the setup specs (:$SETUP_PORT API, :$WIZARD_PORT UI)"
# Own database each, no seeded household, and no FA_ALLOW_PRIVATE_HOSTS (the setup spec checks
# that the guard refuses private addresses by default). Everything else mirrors the main instance.
LAN_IP=$(hostname -I 2>/dev/null | tr ' ' '\n' | grep -E '^(10|192\.168|172\.(1[6-9]|2[0-9]|3[01]))\.' | head -n1 || true)
start_empty() { # name, bind-address, port
  docker exec "$PROJECT-pg" psql -U fa -d postgres -qc "create database fa_$1" >/dev/null
  (
    # The binary loads .env.dev from the working directory; run elsewhere so it cannot leak in.
    cd "$WORK"
    unset FA_ALLOW_PRIVATE_HOSTS FA_COOKIE_SECURE FA_PUBLIC_BASE_URL FA_LAN_DASHBOARD_URL FA_LAN_DASHBOARD_BIND FA_NTFY_URL
    export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PG_PORT/fa_$1" FA_BIND_ADDR="$2:$3"
    "$BIN" migrate >"$WORK/$1.migrate.log" 2>&1
    exec "$BIN" serve api >"$WORK/$1.log" 2>&1
  ) &
  PIDS+=($!)
  for _ in $(seq 60); do curl -fsS "http://127.0.0.1:$3/readyz" >/dev/null 2>&1 && return; sleep 1; done
  echo "empty instance $1 did not start" >&2; tail -n 40 "$WORK/$1.log" >&2; exit 1
}
start_empty setup 127.0.0.1 "$SETUP_PORT"
if [[ -n $LAN_IP ]]; then start_empty wizard 0.0.0.0 "$WIZARD_PORT"; WIZARD_URL=http://$LAN_IP:$WIZARD_PORT
else echo "warning: no private LAN address found; the wizard runs on 127.0.0.1, a secure context" >&2
  start_empty wizard 127.0.0.1 "$WIZARD_PORT"; WIZARD_URL=http://127.0.0.1:$WIZARD_PORT; fi
export FA_E2E_SETUP_URL=http://127.0.0.1:$SETUP_PORT FA_E2E_SETUP_DATABASE_URL="postgres://fa:fa@127.0.0.1:$PG_PORT/fa_setup"

echo "== serve api + serve worker (separate processes)"
"$BIN" serve api >"$WORK/api.log" 2>&1 &
PIDS+=($!)
API_PID=$!
for _ in $(seq 60); do
  curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break
  kill -0 "$API_PID" 2>/dev/null || { dump_logs; echo "api exited" >&2; exit 1; }
  sleep 1
done
curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null || { dump_logs; exit 1; }
"$BIN" serve worker >"$WORK/worker.log" 2>&1 &
PIDS+=($!)

export FA_E2E_BASE_URL=$FA_PUBLIC_BASE_URL
export FA_E2E_FIXTURES="http://127.0.0.1:$STUB_PORT" FA_E2E_RADICALE=$RAD
export FA_E2E_SEED=$WORK/seed.json
export FA_E2E_BIN=$BIN
export FA_E2E_IMAP_PORT=$IMAP_PORT FA_E2E_SMTP_PORT=$SMTP_PORT
export FA_E2E_NTFY=$FA_NTFY_URL FA_E2E_PG_CONTAINER=$PROJECT-pg
export FA_E2E_LAN_URL=$FA_LAN_DASHBOARD_URL
echo "== seed sources, wait for first sync and weather"
node e2e/seed.mjs "$FA_E2E_SEED" || { dump_logs; exit 1; }

if [[ -n ${FA_E2E_HOLD:-} ]]; then echo "stack up for ${FA_E2E_HOLD}s: $FA_E2E_BASE_URL (specs: FA_E2E_SEED=$FA_E2E_SEED FA_E2E_NTFY=$FA_E2E_NTFY FA_E2E_PG_CONTAINER=$FA_E2E_PG_CONTAINER FA_E2E_BIN=$FA_E2E_BIN)"; sleep "$FA_E2E_HOLD"; fi

echo "== playwright (API specs, e2e/)"
cd e2e
corepack pnpm@9 install >/dev/null
corepack pnpm@9 exec playwright install chromium
if ! corepack pnpm@9 exec playwright test; then dump_logs; exit 1; fi
cd ..

echo "== playwright (UI specs, apps/web/e2e/)"
cd apps/web
if ! E2E_BASE_URL=$FA_PUBLIC_BASE_URL E2E_USERNAME=$FA_E2E_USER E2E_PASSWORD=$FA_E2E_PASSWORD \
  E2E_CHILD_USERNAME=$FA_E2E_CHILD_USER E2E_CHILD_PASSWORD=$FA_E2E_CHILD_PASSWORD \
  E2E_FIXTURES=$FA_E2E_FIXTURES E2E_PG_CONTAINER=$FA_E2E_PG_CONTAINER \
  E2E_IMAP_PORT=$IMAP_PORT E2E_SMTP_PORT=$SMTP_PORT E2E_WIZARD_URL=$WIZARD_URL E2E_NTFY_URL=$FA_NTFY_URL \
  corepack pnpm@9 exec playwright test; then dump_logs; exit 1; fi
cd ../..
echo "e2e OK"
