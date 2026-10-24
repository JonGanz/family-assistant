#!/usr/bin/env bash
# Browser smoke test: throwaway Postgres + `fa-server serve all` + Playwright (chromium, headless).
# Env: FA_E2E_PG_PORT (55499), FA_E2E_PORT (18080), FA_E2E_SKIP_BUILD=1 to reuse apps/web/dist.
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD

PG_PORT=${FA_E2E_PG_PORT:-55499}
PORT=${FA_E2E_PORT:-18080}
PROJECT=fa-e2e-$$
WORK=$(mktemp -d)
SERVER_PID=

cleanup() {
  [[ -n $SERVER_PID ]] && kill "$SERVER_PID" 2>/dev/null || true
  docker rm -f -v "$PROJECT-pg" >/dev/null 2>&1 || true
  rm -rf "$WORK"
}
trap cleanup EXIT

export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PG_PORT/fa"
export FA_BIND_ADDR="127.0.0.1:$PORT"
export FA_PUBLIC_BASE_URL="http://127.0.0.1:$PORT"
export FA_COOKIE_SECURE=false
export FA_LOG_LEVEL=info
export FA_LOG_FORMAT=pretty
export FA_MASTER_KEY_PATH="$WORK/master.key"
export FA_WEB_DIST_DIR="$ROOT/apps/web/dist"
head -c 32 /dev/urandom > "$FA_MASTER_KEY_PATH"
chmod 400 "$FA_MASTER_KEY_PATH"

echo "== build"
cargo build -q -p fa-server
if [[ -z ${FA_E2E_SKIP_BUILD:-} || ! -f $FA_WEB_DIST_DIR/index.html ]]; then
  (cd apps/web && corepack pnpm@9 install --frozen-lockfile >/dev/null && corepack pnpm@9 run build >/dev/null)
fi
BIN=$ROOT/target/debug/fa-server

echo "== postgres on :$PG_PORT"
docker run -d --name "$PROJECT-pg" -e POSTGRES_USER=fa -e POSTGRES_PASSWORD=fa -e POSTGRES_DB=fa \
  -p "127.0.0.1:$PG_PORT:5432" pgvector/pgvector:pg16 >/dev/null
for _ in $(seq 60); do
  docker exec "$PROJECT-pg" pg_isready -U fa -d fa >/dev/null 2>&1 && break
  sleep 1
done
sleep 2 # the entrypoint restarts postgres once after init

echo "== seed"
"$BIN" migrate
"$BIN" config import config/examples/household.example.toml
export FA_E2E_USER=alex@example.com FA_E2E_PASSWORD=correct-horse-battery-staple
export FA_E2E_CHILD_USER=riley@example.com FA_E2E_CHILD_PASSWORD=dino1234
printf '%s\n' "$FA_E2E_PASSWORD" | "$BIN" user add --member alex --username "$FA_E2E_USER" --password-stdin
printf '%s\n' "$FA_E2E_CHILD_PASSWORD" | "$BIN" user add --member riley --username "$FA_E2E_CHILD_USER" --password-stdin

echo "== serve all"
"$BIN" serve all >"$WORK/server.log" 2>&1 &
SERVER_PID=$!
for _ in $(seq 60); do
  curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break
  kill -0 "$SERVER_PID" 2>/dev/null || { cat "$WORK/server.log"; echo "server exited" >&2; exit 1; }
  sleep 1
done
curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null || { cat "$WORK/server.log"; exit 1; }

echo "== playwright"
export FA_E2E_BASE_URL=$FA_PUBLIC_BASE_URL
cd e2e
corepack pnpm@9 install >/dev/null
corepack pnpm@9 exec playwright install chromium
if ! corepack pnpm@9 exec playwright test; then
  echo "---- server log ----"; tail -n 60 "$WORK/server.log"
  exit 1
fi
cd ..

echo "== playwright (web UI smoke, apps/web/e2e)"
cd apps/web
if ! E2E_BASE_URL=$FA_PUBLIC_BASE_URL E2E_USERNAME=$FA_E2E_USER E2E_PASSWORD=$FA_E2E_PASSWORD \
  corepack pnpm@9 exec playwright test; then
  echo "---- server log ----"; tail -n 60 "$WORK/server.log"
  exit 1
fi
cd ../..
echo "e2e OK"
