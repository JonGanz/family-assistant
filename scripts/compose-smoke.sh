#!/usr/bin/env bash
# Boots deploy/compose.example.yaml (default profile) with throwaway secrets, checks /readyz,
# triggers an on-demand backup and checks a dump appears, then tears everything down.
# Leaves $FA_SMOKE_OUT (default /tmp/fa-smoke-out) with fa.dump, master.key and members.count
# for scripts/restore-rehearsal.sh.
# Env: FA_SMOKE_IMAGE=<local image> to skip the build (it is tagged as the compose image).
# Host ports: FA_SMOKE_API_PORT (18180) and FA_SMOKE_NTFY_PORT (18190), so a dev server on 8080 is left alone.
set -euo pipefail
cd "$(dirname "$0")/.."

PROJECT=fa-smoke
API_PORT=${FA_SMOKE_API_PORT:-18180}
NTFY_PORT=${FA_SMOKE_NTFY_PORT:-18190}
OUT=${FA_SMOKE_OUT:-/tmp/fa-smoke-out}
WORK=$(mktemp -d)
BACKUP_DIR=$WORK/backups
mkdir -p "$BACKUP_DIR"
chmod 777 "$BACKUP_DIR" # the backup container writes as its own user

head -c 32 /dev/urandom > "$WORK/master.key"
chmod 444 "$WORK/master.key" # readable by the container user (uid 10001); throwaway key
cat > "$WORK/test.env" <<ENV
FA_VERSION=smoke
FA_IMAGE_OWNER=local
POSTGRES_USER=fa
POSTGRES_PASSWORD=smoke-$(head -c 6 /dev/urandom | od -An -tx1 | tr -d ' \n')
POSTGRES_DB=fa
FA_MASTER_KEY_FILE=$WORK/master.key
BACKUP_DIR=$BACKUP_DIR
FA_PUBLIC_BASE_URL=http://127.0.0.1:$API_PORT
FA_HOST_API_PORT=$API_PORT
FA_HOST_NTFY_PORT=$NTFY_PORT
FA_COOKIE_SECURE=false
FA_LOG_FORMAT=pretty
NTFY_BASE_URL=http://localhost:$NTFY_PORT
ENV

dc() { docker compose -p "$PROJECT" --env-file "$WORK/test.env" -f deploy/compose.example.yaml "$@"; }

cleanup() {
  dc logs --tail 40 api 2>/dev/null | sed 's/^/[api] /' >&2 || true
  dc down -v --remove-orphans >/dev/null 2>&1 || true
  # backup files are owned by the container user
  docker run --rm -v "$WORK:/w" --entrypoint sh prodrigestivill/postgres-backup-local:16 -c 'rm -rf /w/backups' >/dev/null 2>&1 || true
  rm -rf "$WORK"
  [[ -z ${FA_SMOKE_IMAGE:-} ]] && docker image rm ghcr.io/local/fa-server:smoke >/dev/null 2>&1 || true
}
trap cleanup EXIT

for p in $API_PORT $NTFY_PORT; do
  if ss -ltn | awk '{print $4}' | grep -qE "[:.]$p$"; then
    echo "host port $p is in use; stop whatever holds it and retry" >&2
    exit 1
  fi
done

# ntfy is left out unless FA_SMOKE_NTFY=1: it is not exercised here.
SERVICES=(postgres api worker backup)
[[ -n ${FA_SMOKE_NTFY:-} ]] && SERVICES+=(ntfy)
if [[ -n ${FA_SMOKE_IMAGE:-} ]]; then
  docker tag "$FA_SMOKE_IMAGE" ghcr.io/local/fa-server:smoke
  dc up -d --no-build "${SERVICES[@]}"
else
  dc up -d --build "${SERVICES[@]}"
fi

echo "== waiting for /readyz"
for _ in $(seq 90); do
  curl -fsS http://127.0.0.1:$API_PORT/readyz >/dev/null 2>&1 && break
  sleep 2
done
curl -fsS http://127.0.0.1:$API_PORT/readyz >/dev/null || { echo "readyz never became OK" >&2; exit 1; }
echo "readyz OK"

echo "== seed household"
dc cp config/examples/household.example.toml api:/tmp/household.toml
dc exec -T api fa-server config import /tmp/household.toml >/dev/null
MEMBERS=$(dc exec -T postgres psql -U fa -d fa -Atc "select count(*) from member")
echo "members: $MEMBERS"

echo "== on-demand backup"
dc exec -T backup /backup.sh
DUMP=$(find "$BACKUP_DIR/last" -type f -size +0 ! -name '*latest*' 2>/dev/null | sort | tail -n1)
[[ -n $DUMP ]] || { echo "no dump file appeared in BACKUP_DIR" >&2; find "$BACKUP_DIR" >&2; exit 1; }
echo "dump: $(basename "$DUMP") ($(stat -c %s "$DUMP") bytes)"

mkdir -p "$OUT"
cp -f "$DUMP" "$OUT/fa.dump"
cp -f "$WORK/master.key" "$OUT/master.key"
echo "$MEMBERS" > "$OUT/members.count"
echo "compose-smoke OK (artifacts in $OUT)"
