set dotenv-filename := ".env.dev"
set dotenv-load

compose := "docker compose -f deploy/compose.dev.yaml"

# Start the dev Postgres and wait until healthy
dev-db:
    {{compose}} up -d --wait postgres

# Dev DB, api/worker with auto-reload, and the Vite dev server
dev: dev-db
    #!/usr/bin/env bash
    set -euo pipefail
    trap 'kill 0' EXIT
    cargo watch -x 'run -p fa-server -- serve all' &
    (cd apps/web && corepack pnpm@9 dev) &
    wait

test:
    cargo nextest run --workspace
    cd apps/web && corepack pnpm@9 test

lint:
    cargo fmt --check
    cargo clippy --workspace --all-targets -- -D warnings
    cargo deny check
    cd apps/web && corepack pnpm@9 lint

# Regenerate TS types (ts-rs) and the OpenAPI client
gen:
    @echo "TODO: wired up by the api and web tasks"

e2e:
    @echo "TODO: Playwright against compose, wired up later in M0"
