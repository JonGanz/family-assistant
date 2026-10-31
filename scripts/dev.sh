#!/usr/bin/env bash
# Clone to running app: services, migrations, sample data (first time only), then the app with HMR.
set -euo pipefail
cd "$(dirname "$0")/.."
scripts/dev-up.sh
scripts/dev-seed.sh
exec scripts/dev-run.sh
