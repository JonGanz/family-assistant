#!/usr/bin/env bash
# Boots the install file the way a user does: deploy/compose.yaml (plus deploy/compose.build.yaml
# so it builds from this checkout) with NO .env and no settings except the host ports. Checks:
#   - `init` created the secrets with the right modes and owners, and a second `up` (and a second
#     run of `init`) leaves them untouched;
#   - /readyz, the first-run setup through the API, and a login cookie without `Secure` (plain HTTP);
#   - the backup service authenticates with the generated password and writes a dump;
#   - `docker compose --profile tailscale config` (and gpu, ai-cpu) validate;
# then tears everything down with `down -v`.
# Leaves $FA_SMOKE_OUT (default /tmp/fa-smoke-out) with fa.dump, master.key and members.count
# for scripts/restore-rehearsal.sh.
# Env: FA_SMOKE_IMAGE=<local image> to skip the build (it is tagged as the compose image).
# Host ports: FA_SMOKE_API_PORT (18180) and FA_SMOKE_NTFY_PORT (18190), so a dev server on 8080
# is left alone.
set -euo pipefail
cd "$(dirname "$0")/.."

PROJECT=fa-smoke
API_PORT=${FA_SMOKE_API_PORT:-18180}
NTFY_PORT=${FA_SMOKE_NTFY_PORT:-18190}
OUT=${FA_SMOKE_OUT:-/tmp/fa-smoke-out}
WORK=$(mktemp -d)
URL=http://127.0.0.1:$API_PORT
HELPER_IMAGE=pgvector/pgvector:pg16 # already pulled for the stack; has stat, sha256sum, find

# No .env: nothing from the caller's environment or a stray deploy/.env reaches the stack.
[[ -e deploy/.env ]] && { echo "deploy/.env exists: remove it, this smoke test must run without one" >&2; exit 1; }
unset FA_HTTP_HOST_IP BACKUP_DIR NTFY_BASE_URL NTFY_UPSTREAM_BASE_URL TS_AUTHKEY TS_HOSTNAME OLLAMA_BIND FA_LOG_LEVEL
export FA_VERSION=smoke FA_HTTP_PORT=$API_PORT FA_NTFY_PORT=$NTFY_PORT

dc() {
  docker compose -p "$PROJECT" --env-file /dev/null \
    -f deploy/compose.yaml -f deploy/compose.build.yaml "$@"
}
vol() { echo "${PROJECT}_$1"; }
# Run a throwaway helper with a named volume mounted read-only at /v.
helper() { docker run --rm -v "$(vol "$1"):/v:ro" --entrypoint "${@:2}"; }
fail() { echo "FAIL: $*" >&2; exit 1; }

cleanup() {
  local rc=$?
  if [[ $rc -ne 0 ]]; then
    dc logs --tail 40 app init postgres backup 2>/dev/null | sed 's/^/[stack] /' >&2 || true
  fi
  dc --profile tailscale --profile gpu --profile ai-cpu down -v --remove-orphans >/dev/null 2>&1 || true
  rm -rf "$WORK"
  # Only the smoke tag goes: with FA_SMOKE_IMAGE it is an extra name for the caller's image.
  docker image rm "ghcr.io/jonganz/fa-server:smoke" >/dev/null 2>&1 || true
  exit $rc
}
trap cleanup EXIT

for p in $API_PORT $NTFY_PORT; do
  if ss -ltn | awk '{print $4}' | grep -qE "[:.]$p$"; then
    echo "host port $p is in use; stop whatever holds it and retry" >&2
    exit 1
  fi
done

echo "== config: the default file and every profile validate"
dc config -q
dc --profile tailscale config -q
dc --profile gpu config -q
dc --profile ai-cpu config -q
# The tailscale profile adds exactly the sidecar; without it nothing Tailscale runs.
dc config --services | grep -qx tailscale && fail "the tailscale service is on without its profile"
dc --profile tailscale config --services | grep -qx tailscale || fail "the tailscale profile has no sidecar"

echo "== up (no .env)"
if [[ -n ${FA_SMOKE_IMAGE:-} ]]; then
  docker tag "$FA_SMOKE_IMAGE" ghcr.io/jonganz/fa-server:smoke
  dc up -d --no-build
else
  dc up -d --build
fi

echo "== init: secrets exist with the right modes and owners"
INIT_CID=$(dc ps -a -q init)
[[ -n $INIT_CID ]] || fail "no init container"
[[ $(docker inspect -f '{{.State.ExitCode}}' "$INIT_CID") == 0 ]] || fail "init did not exit 0"
stat_secrets() { helper fa-secrets stat "$HELPER_IMAGE" -c '%a %u %n' /v/master_key /v/db_password; }
STAT=$(stat_secrets)
echo "$STAT"
grep -qx '400 10001 /v/master_key' <<<"$STAT" || fail "master_key must be mode 0400, owner 10001"
grep -qx '444 10001 /v/db_password' <<<"$STAT" || fail "db_password must be mode 0444, owner 10001"
[[ $(helper fa-secrets stat "$HELPER_IMAGE" -c %s /v/master_key) == 32 ]] || fail "master_key must be 32 bytes"
sums() { helper fa-secrets sha256sum "$HELPER_IMAGE" /v/master_key /v/db_password; }
SUMS_BEFORE=$(sums)

echo "== readyz"
for _ in $(seq 90); do
  curl -fsS "$URL/readyz" >/dev/null 2>&1 && break
  sleep 2
done
curl -fsS "$URL/readyz" >/dev/null || fail "readyz never became OK"
echo "readyz OK"
curl -fsS "$URL/" | grep -qi "<html" || fail "the web app is not served at /"
curl -fsS "http://127.0.0.1:$NTFY_PORT/v1/health" | grep -q '"healthy":true' || fail "ntfy is not healthy"

echo "== init is idempotent: a second up and a second run keep the secrets"
dc up -d --no-build >/dev/null
dc run --rm --no-deps init >/dev/null
[[ $(sums) == "$SUMS_BEFORE" ]] || fail "init changed existing secrets"
[[ $(stat_secrets) == "$STAT" ]] || fail "init changed modes or owners"
curl -fsS "$URL/readyz" >/dev/null || fail "readyz failed after the second up"

echo "== first-run setup through the API"
api() { curl -sS -H 'X-FA-Request: 1' -H 'Content-Type: application/json' "$@"; }
[[ $(api "$URL/api/v1/setup" | grep -o '"needed":[a-z]*') == '"needed":true' ]] || fail "a fresh install must need setup"
[[ $(api -o /dev/null -w '%{http_code}' "$URL/api/v1/me") == 409 ]] || fail "/me must answer 409 before setup"
CODE=$(api -o "$WORK/household.json" -w '%{http_code}' -X POST "$URL/api/v1/setup/household" \
  -d "{\"name\":\"Smoke household\",\"tz\":\"UTC\",\"units\":\"metric\",\"currency\":\"USD\",\"base_url\":\"$URL\"}")
[[ $CODE == 201 ]] || fail "setup/household answered $CODE: $(cat "$WORK/household.json")"
PASSWORD=smoke-test-passphrase-1
CODE=$(api -D "$WORK/owner.headers" -o "$WORK/owner.json" -w '%{http_code}' -X POST "$URL/api/v1/setup/owner" \
  -d "{\"display_name\":\"Pat\",\"username\":\"pat@example.com\",\"password\":\"$PASSWORD\"}")
[[ $CODE == 201 ]] || fail "setup/owner answered $CODE: $(cat "$WORK/owner.json")"
COOKIE_LINE=$(grep -i '^set-cookie:' "$WORK/owner.headers" | head -n1 | tr -d '\r')
[[ -n $COOKIE_LINE ]] || fail "setup/owner set no session cookie"
grep -qi ';[[:space:]]*Secure' <<<"$COOKIE_LINE" && fail "the cookie is Secure over plain HTTP: $COOKIE_LINE"
SESSION=$(sed -E 's/^[Ss]et-[Cc]ookie: *([^;]*).*/\1/' <<<"$COOKIE_LINE")
[[ $(api -o /dev/null -w '%{http_code}' -H "Cookie: $SESSION" "$URL/api/v1/me") == 200 ]] || fail "the new session does not work"
[[ $(api -o /dev/null -w '%{http_code}' -X POST -H "Cookie: $SESSION" "$URL/api/v1/setup/complete") == 204 ]] || fail "setup/complete failed"
[[ $(api -o /dev/null -w '%{http_code}' -X POST "$URL/api/v1/setup/owner" \
  -d '{"display_name":"Eve","username":"eve@example.com","password":"another-passphrase-2"}') == 409 ]] || fail "setup must close after the first adult"
# A login over plain HTTP also gets a cookie without Secure.
LOGIN_HEADERS=$(api -D - -o /dev/null -X POST "$URL/api/v1/auth/login" -d "{\"username\":\"pat@example.com\",\"password\":\"$PASSWORD\"}" | tr -d '\r')
grep -qi '^set-cookie:' <<<"$LOGIN_HEADERS" || fail "login set no cookie"
grep -i '^set-cookie:' <<<"$LOGIN_HEADERS" | grep -qi ';[[:space:]]*Secure' && fail "the login cookie is Secure over plain HTTP"
echo "setup OK, cookie without Secure"

echo "== the key the API shows is the key on disk"
KEY_B64=$(api -X POST -H "Cookie: $SESSION" "$URL/api/v1/system/master-key" -d "{\"password\":\"$PASSWORD\"}" | sed -E 's/.*"key_base64":"([^"]*)".*/\1/')
mkdir -p "$OUT"
rm -f "$OUT/master.key" "$OUT/fa.dump" "$OUT/members.count"
helper fa-secrets cat "$HELPER_IMAGE" /v/master_key > "$OUT/master.key"
[[ $(base64 -w0 "$OUT/master.key") == "$KEY_B64" ]] || fail "the key from the API differs from the key file"
chmod 600 "$OUT/master.key"

echo "== on-demand backup (the backup service reads the generated password)"
dc exec -T backup /backup.sh >/dev/null
helper fa-backups sh "$HELPER_IMAGE" -c 'find /v/last -type f -size +0 ! -name "*latest*" | sort | tail -n1' > "$WORK/dump.path"
DUMP_PATH=$(<"$WORK/dump.path")
[[ -n $DUMP_PATH ]] || fail "no dump file appeared in the backup volume"
docker run --rm -v "$(vol fa-backups):/v:ro" -v "$OUT:/out" --entrypoint sh "$HELPER_IMAGE" \
  -c "cp '$DUMP_PATH' /out/fa.dump && chmod a+rw /out/fa.dump"
[[ -s $OUT/fa.dump ]] || fail "the dump is empty"
echo "dump: $(basename "$DUMP_PATH") ($(stat -c %s "$OUT/fa.dump") bytes)"
dc exec -T postgres psql -U fa -d fa -Atc "select count(*) from member" > "$OUT/members.count"
echo "members: $(<"$OUT/members.count")"

echo "== down -v"
dc --profile tailscale --profile gpu --profile ai-cpu down -v --remove-orphans >/dev/null
if docker volume ls -q | grep -q "^${PROJECT}_"; then fail "volumes of $PROJECT are left after down -v"; fi
echo "compose-smoke OK (artifacts in $OUT)"
