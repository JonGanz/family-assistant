#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally, against a FRESH throwaway Postgres
# (never the dev DB: its state can hide failures). Usage: scripts/ci-local.sh [--no-web]
set -euo pipefail
cd "$(dirname "$0")/.."

PORT=55496
NAME=fa-ci-local-$$
cleanup() { docker rm -f "$NAME" >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "== fresh postgres on :$PORT"
docker run -d --rm --name "$NAME" -e POSTGRES_USER=fa -e POSTGRES_PASSWORD=fa -e POSTGRES_DB=fa \
  -p "127.0.0.1:$PORT:5432" pgvector/pgvector:pg16 >/dev/null
for _ in $(seq 30); do docker exec "$NAME" pg_isready -U fa >/dev/null 2>&1 && break; sleep 1; done

export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PORT/fa" SQLX_OFFLINE=true

echo "== rust: fmt, clippy, tests"
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
if cargo nextest --version >/dev/null 2>&1; then cargo nextest run --workspace; else cargo test --workspace; fi

echo "== deny"
if cargo deny --version >/dev/null 2>&1; then cargo deny check; else echo "cargo-deny not installed, skipped (cargo install cargo-deny --locked)"; fi

echo "== hygiene"
scripts/check-example-data.sh
if command -v gitleaks >/dev/null; then gitleaks detect --no-banner; else echo "gitleaks not installed, skipped (runs in CI)"; fi

if [ "${1:-}" != "--no-web" ]; then
  echo "== web: lint, typecheck, test, build"
  cd apps/web
  corepack pnpm@9 install --frozen-lockfile
  for s in lint typecheck test build; do corepack pnpm@9 run "$s"; done
fi

echo "ci-local OK"
