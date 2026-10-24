set dotenv-filename := ".env.dev"
set dotenv-load

compose := "docker compose -f deploy/compose.dev.yaml"

# Start the dev Postgres and wait until healthy
dev-db:
    {{compose}} up -d --wait postgres

# Dev DB, api/worker with auto-reload, and the Vite dev server
dev: dev-db dev-key
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

# Create the local dev master key (.dev/master.key)
dev-key:
    scripts/dev-key.sh

# Build the web app and run api + worker on http://localhost:8080
serve: dev-db dev-key
    cd apps/web && corepack pnpm@9 install && corepack pnpm@9 run build
    cargo run -p fa-server -- serve all

# Regenerate TS types (ts-rs), the OpenAPI document and the typed web client
gen:
    cargo test -q -p fa-domain
    cargo run -q -p fa-server -- gen-openapi --out apps/web/src/gen/openapi.json
    cd apps/web && corepack pnpm@9 run gen

# Same checks as CI, against a fresh throwaway Postgres
ci:
    scripts/ci-local.sh

# Browser smoke tests (throwaway Postgres + serve all + Playwright)
e2e:
    scripts/e2e.sh
