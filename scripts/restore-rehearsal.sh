#!/usr/bin/env bash
# Usage: scripts/restore-rehearsal.sh <dump> [master-key-file]
# Restores a backup into a fresh throwaway Postgres, starts the api against it with the same
# master key, checks /readyz, and checks the member count.
# Master key: 2nd arg, else $FA_MASTER_KEY_FILE, else master.key next to the dump.
# Expected members: $FA_EXPECT_MEMBERS, else members.count next to the dump, else just > 0.
# Server binary: $FA_SERVER_BIN (default target/debug/fa-server, built if missing).
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD

DUMP=${1:?usage: $0 <dump> [master-key-file]}
DUMP=$(realpath "$DUMP")
KEY=${2:-${FA_MASTER_KEY_FILE:-$(dirname "$DUMP")/master.key}}
[[ -f $KEY ]] || { echo "master key not found: $KEY" >&2; exit 1; }
EXPECT=${FA_EXPECT_MEMBERS:-}
[[ -z $EXPECT && -f $(dirname "$DUMP")/members.count ]] && EXPECT=$(<"$(dirname "$DUMP")/members.count")

BIN=${FA_SERVER_BIN:-$ROOT/target/debug/fa-server}
[[ -x $BIN ]] || { cargo build -q -p fa-server; }

PG_PORT=${FA_REHEARSAL_PG_PORT:-55498}
PORT=${FA_REHEARSAL_PORT:-18081}
NAME=fa-restore-$$
LOG=$(mktemp)
SERVER_PID=
cleanup() {
  [[ -n $SERVER_PID ]] && kill "$SERVER_PID" 2>/dev/null || true
  docker rm -f -v "$NAME" >/dev/null 2>&1 || true
  rm -f "$LOG"
}
trap cleanup EXIT

docker run -d --name "$NAME" -e POSTGRES_USER=fa -e POSTGRES_PASSWORD=fa -e POSTGRES_DB=fa \
  -p "127.0.0.1:$PG_PORT:5432" pgvector/pgvector:pg16 >/dev/null
for _ in $(seq 60); do
  docker exec "$NAME" pg_isready -U fa -d fa >/dev/null 2>&1 && break
  sleep 1
done
sleep 2

# Roles live in the cluster, not in the dump: a fresh cluster needs fa_app (see migration 0001)
# before the restore, and the login user must be a member of it for SET ROLE.
docker exec "$NAME" psql -U fa -d fa -v ON_ERROR_STOP=1 -q \
  -c "CREATE ROLE fa_app NOLOGIN NOSUPERUSER NOBYPASSRLS" -c "GRANT fa_app TO fa"

echo "== restore $(basename "$DUMP")"
if [[ $(head -c 5 "$DUMP") == PGDMP ]]; then
  docker exec -i "$NAME" pg_restore -U fa -d fa --no-owner --exit-on-error < "$DUMP"
elif gzip -t "$DUMP" 2>/dev/null; then
  gunzip -c "$DUMP" | docker exec -i "$NAME" psql -U fa -d fa -v ON_ERROR_STOP=1 -q >/dev/null
else
  docker exec -i "$NAME" psql -U fa -d fa -v ON_ERROR_STOP=1 -q < "$DUMP" >/dev/null
fi

export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PG_PORT/fa"
export FA_BIND_ADDR="127.0.0.1:$PORT"
export FA_PUBLIC_BASE_URL="http://127.0.0.1:$PORT"
export FA_COOKIE_SECURE=false
export FA_MASTER_KEY_PATH=$KEY
export FA_LOG_FORMAT=pretty

echo "== serve api against the restored database"
"$BIN" serve api >"$LOG" 2>&1 &
SERVER_PID=$!
for _ in $(seq 60); do
  curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null 2>&1 && break
  kill -0 "$SERVER_PID" 2>/dev/null || { cat "$LOG" >&2; echo "api exited" >&2; exit 1; }
  sleep 1
done
curl -fsS "http://127.0.0.1:$PORT/readyz" >/dev/null || { cat "$LOG" >&2; echo "readyz failed" >&2; exit 1; }
echo "readyz OK"

COUNT=$(docker exec "$NAME" psql -U fa -d fa -Atc "select count(*) from member")
echo "members in restored database: $COUNT"
if [[ -n $EXPECT ]]; then
  [[ $COUNT == "$EXPECT" ]] || { echo "member count mismatch: expected $EXPECT, got $COUNT" >&2; exit 1; }
else
  [[ $COUNT -gt 0 ]] || { echo "restored database has no members" >&2; exit 1; }
fi
echo "restore-rehearsal OK"
