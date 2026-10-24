#!/usr/bin/env bash
# Create a throwaway master key for local development at .dev/master.key (git-ignored).
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p .dev
if [ -f .dev/master.key ]; then echo ".dev/master.key already exists"; exit 0; fi
head -c 32 /dev/urandom > .dev/master.key
chmod 400 .dev/master.key
echo "created .dev/master.key (dev only, never use in production)"
