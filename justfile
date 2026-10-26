set dotenv-filename := ".env.dev"
set dotenv-load

compose := "docker compose -f deploy/compose.dev.yaml"

# Start the dev Postgres and ntfy and wait until healthy
dev-db:
    {{compose}} up -d --wait postgres ntfy

# Start a local GreenMail (IMAP 127.0.0.1:13143, SMTP 127.0.0.1:13025) for trying the mail features
dev-mail:
    {{compose}} up -d greenmail

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

# Build the web app and run `serve api` and `serve worker` as two separate processes
serve-split: dev-db dev-key
    #!/usr/bin/env bash
    set -euo pipefail
    (cd apps/web && corepack pnpm@9 install && corepack pnpm@9 run build)
    cargo build -p fa-server
    trap 'kill 0' EXIT
    target/debug/fa-server serve api &
    target/debug/fa-server serve worker &
    wait

# Same checks as CI, against a fresh throwaway Postgres
ci:
    scripts/ci-local.sh

# End-to-end tests (throwaway Postgres, api + worker, fixture feeds, Radicale, Playwright)
e2e:
    scripts/e2e.sh

# Wipe the dev DB and rebuild the example household, fixtures and a paired device, then serve
demo-reset:
    scripts/demo-reset.sh
