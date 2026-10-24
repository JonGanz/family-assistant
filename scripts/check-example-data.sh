#!/usr/bin/env bash
# Fail if any email domain in example/fixture/e2e data is not a reserved one (D-035).
set -euo pipefail
cd "$(git rev-parse --show-toplevel 2>/dev/null || dirname "$0"/..)"

allowed='^(([a-z0-9-]+\.)*example\.(com|org|net)|[a-z0-9.-]+\.(test|invalid))$'
status=0

while IFS= read -r -d '' f; do
  while IFS= read -r m; do
    domain=${m#*@}
    domain=$(printf '%s' "$domain" | tr 'A-Z' 'a-z')
    if ! [[ $domain =~ $allowed ]]; then
      echo "$f: disallowed email domain: $m" >&2
      status=1
    fi
  done < <(grep -aEoh '[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(\.[A-Za-z0-9-]+)+' "$f" || true)
done < <(find . \
  \( -name node_modules -o -name target -o -name .git \) -prune -o \
  -type f ! -name 'pnpm-lock.yaml' ! -name 'package-lock.json' ! -name 'package.json' ! -name 'Cargo.lock' \( -path './config/examples/*' -o -path '*/fixtures/*' -o -path './e2e/*' -o -path '*/e2e/*' \) \
  -print0)

if [ "$status" -eq 0 ]; then echo "check-example-data: ok"; fi
exit "$status"
