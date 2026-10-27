#!/usr/bin/env bash
# Run the same checks as .github/workflows/ci.yml locally, against a FRESH throwaway Postgres
# (never the dev DB: its state can hide failures). Usage: scripts/ci-local.sh [--no-web]
set -euo pipefail
cd "$(dirname "$0")/.."

echo "== docs: front-matter and index check, its negative tests, YAML"
python3 tools/fa.py check
python3 tools/test_fa.py
python3 tools/fa.py check-yaml .github/workflows/*.yml deploy/*.yaml

PORT=55496
IMAP_PORT=55497
IMAPS_PORT=55498
NAME=fa-ci-local-$$
cleanup() { docker rm -f -v "$NAME" "$NAME-greenmail" >/dev/null 2>&1 || true; }
trap cleanup EXIT

echo "== fresh postgres on :$PORT"
docker run -d --rm --name "$NAME" -e POSTGRES_USER=fa -e POSTGRES_PASSWORD=fa -e POSTGRES_DB=fa \
  -p "127.0.0.1:$PORT:5432" pgvector/pgvector:pg16 >/dev/null
for _ in $(seq 30); do docker exec "$NAME" pg_isready -U fa >/dev/null 2>&1 && break; sleep 1; done

echo "== greenmail on :$IMAP_PORT/:$IMAPS_PORT (for the IMAP tests)"
docker run -d --name "$NAME-greenmail" -p "127.0.0.1:$IMAP_PORT:3143" -p "127.0.0.1:$IMAPS_PORT:3993" \
  -e GREENMAIL_OPTS='-Dgreenmail.setup.test.imap -Dgreenmail.setup.test.imaps -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled' \
  greenmail/standalone:2.1.8 >/dev/null
# Docker's port proxy accepts connections before GreenMail does: wait for the IMAP greeting.
for _ in $(seq 60); do
  (exec 3<>"/dev/tcp/127.0.0.1/$IMAP_PORT" && read -r -t 2 greeting <&3 && [[ $greeting == "* OK"* ]]) 2>/dev/null && break
  sleep 1
done

export DATABASE_URL="postgres://fa:fa@127.0.0.1:$PORT/fa" SQLX_OFFLINE=true

echo "== rust: fmt, clippy, tests"
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
if cargo nextest --version >/dev/null 2>&1; then cargo nextest run --workspace; else cargo test --workspace; fi
echo "== rust: the GreenMail-gated IMAP tests (FA_TEST_GREENMAIL)"
export FA_TEST_GREENMAIL="127.0.0.1:$IMAP_PORT:$IMAPS_PORT"
if cargo nextest --version >/dev/null 2>&1; then
  cargo nextest run -p fa-connectors -p fa-mail --run-ignored only -E 'binary(imap) | binary(sync_imap)'
else
  cargo test -p fa-connectors --test imap -- --include-ignored
  cargo test -p fa-mail --test sync_imap -- --include-ignored
fi
unset FA_TEST_GREENMAIL

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
