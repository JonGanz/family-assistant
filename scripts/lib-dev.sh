# Shared helpers for scripts/dev-*.sh (sourced, not run). Expects to be sourced from the repo root.
COMPOSE="docker compose -f deploy/compose.dev.yaml"
PNPM="corepack pnpm@9"
API_URL="http://127.0.0.1:8080"
WEB_URL="http://localhost:5173"
FIXTURE_PORT=18091
export FA_OPEN_METEO_URL="http://127.0.0.1:$FIXTURE_PORT"

need() { command -v "$1" >/dev/null 2>&1 || { echo "missing prerequisite: $1 ($2)" >&2; exit 1; }; }

api_up() { curl -fsS "$API_URL/readyz" >/dev/null 2>&1; }

wait_api() {
  for _ in $(seq 180); do api_up && return 0; sleep 1; done
  echo "the API did not become ready on $API_URL" >&2; return 1
}

# Offline fixtures: fake weather and sample ICS calendars (e2e/stubs/server.mjs).
start_fixtures() {
  node e2e/stubs/server.mjs "$FIXTURE_PORT" >/dev/null 2>&1 &
}
