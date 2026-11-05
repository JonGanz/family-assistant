#!/usr/bin/env bash
# Review screenshots of the UI (M4). Output: apps/web/test-results/ui-shots/<page>-<width>-<light|dark>.png
#   scripts/ui-shots.sh [--gallery]   the dev-only /ui-gallery on a Vite dev server (no backend needed)
#   scripts/ui-shots.sh --all         the gallery plus every app route, against the e2e stack
#                                     (starts it through scripts/e2e.sh, which needs Docker and a Rust build)
set -euo pipefail
cd "$(dirname "$0")/.."
ROOT=$PWD
MODE=--gallery
[[ ${1:-} == --all || ${1:-} == --gallery ]] && MODE=$1
[[ $# -le 1 ]] || { echo "usage: $0 [--gallery|--all]" >&2; exit 2; }
OUT=$ROOT/apps/web/test-results/ui-shots

if [[ $MODE == --all ]]; then
  # Reuse scripts/e2e.sh unchanged: it seeds the stack, then runs `playwright test` in apps/web.
  # PATH gets a corepack shim that skips the API specs and turns the UI-spec run into the @shots run.
  SHIM=$(mktemp -d)
  trap 'rm -rf "$SHIM"' EXIT
  cat >"$SHIM/corepack" <<SH
#!/usr/bin/env bash
if [[ "\$*" == *"exec playwright test"* && "\$PWD" == "$ROOT/e2e" ]]; then
  exit 0 # the API specs are not needed for screenshots
fi
if [[ "\$*" == *"exec playwright test"* && "\$PWD" == "$ROOT/apps/web" ]]; then
  rm -rf "$OUT"
  UI_SHOTS=1 UI_SHOTS_ALL=1 exec "$(command -v corepack)" "\$@" --grep @shots
fi
exec "$(command -v corepack)" "\$@"
SH
  chmod +x "$SHIM/corepack"
  PATH=$SHIM:$PATH "$ROOT/scripts/e2e.sh"
else
  PORT=${UI_SHOTS_PORT:-$(python3 -c 'import socket; s=socket.socket(); s.bind(("127.0.0.1",0)); print(s.getsockname()[1])')}
  cd apps/web
  corepack pnpm@9 install --frozen-lockfile >/dev/null
  corepack pnpm@9 exec playwright install chromium >/dev/null
  rm -rf "$OUT"
  corepack pnpm@9 exec vite --port "$PORT" --strictPort --host 127.0.0.1 >"${TMPDIR:-/tmp}/ui-shots-vite.log" 2>&1 &
  VITE=$!
  trap 'kill $VITE 2>/dev/null || true' EXIT
  for _ in $(seq 60); do curl -fsS "http://127.0.0.1:$PORT/ui-gallery" >/dev/null 2>&1 && break; sleep 1; done
  E2E_BASE_URL=http://127.0.0.1:$PORT UI_SHOTS=1 corepack pnpm@9 exec playwright test --grep @shots --workers=2
fi
echo "screenshots: $OUT"
echo "files: $(find "$OUT" -name '*.png' | wc -l)"
