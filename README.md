# Family Assistant

A self-hosted organiser that takes the mental load off a busy household. It pulls together the
family's email, calendars, weather, chores, homework, meals and groceries, and shows what matters
on a wall dashboard and on everyone's phone.

Everything runs locally. Deterministic rules do most of the work, and local AI models help only
where rules can't decide.

> **Status:** M0, the platform skeleton, is code-complete. Login, members, dashboard device pairing
> with PIN elevation, the job queue, backups and CI work. Feature milestones come next (see
> [Roadmap](#roadmap)).

## Contents
- [Quick start (local development)](#quick-start-local-development)
- [Day-to-day development](#day-to-day-development)
- [Checks: run CI locally](#checks-run-ci-locally)
- [Deploying](#deploying)
- [Repository layout](#repository-layout)
- [How this project is designed](#how-this-project-is-designed)
- [Roadmap](#roadmap)
- [License](#license)

## Quick start (local development)

### Prerequisites
| Tool | Version | Notes |
|---|---|---|
| Rust | per `rust-toolchain.toml` | `rustup` installs it automatically |
| Node.js | 24 | pnpm comes through **corepack** (`corepack pnpm@9 …`); nothing needs a global install |
| Docker + Compose v2 | recent | dev Postgres, e2e tests, deployment |
| [`just`](https://github.com/casey/just) | optional | shortcuts only. Every recipe is a thin wrapper, so each step below also shows the plain command |

### 1. Start the dev database and create a dev master key
```bash
just dev-db dev-key
# or, without just:
docker compose -f deploy/compose.dev.yaml up -d --wait postgres
scripts/dev-key.sh
```
The dev database listens on `127.0.0.1:55432` (user, password and database are all `fa`). The dev
key goes to `.dev/master.key`, which is git-ignored. Local settings live in [`.env.dev`](.env.dev).

### 2. Load the example household and create accounts
`config/examples/household.example.toml` is a fictional family. Load it, then give a member a login:
```bash
set -a; . ./.env.dev; set +a          # just loads .env.dev itself; plain shells need this

cargo run -p fa-server -- config import config/examples/household.example.toml
cargo run -p fa-server -- user add --member alex --username alex@example.com   # prompts for a password (adults: 12+ chars)
cargo run -p fa-server -- user add --member riley --username riley@example.com # child (4+ chars)
cargo run -p fa-server -- pin set --member alex                                 # 4–6 digit PIN for dashboard approvals
```
Forgotten a password? Run `cargo run -p fa-server -- user reset-password --username alex@example.com`.

### 3. Run it
```bash
just serve
# or, without just:
(cd apps/web && corepack pnpm@9 install && corepack pnpm@9 run build)
cargo run -p fa-server -- serve all
```
Open <http://localhost:8080> and log in.

To try the **dashboard**, open *Devices*, pair a device, and open the pairing link (or scan the QR
code). It opens `/dashboard?device=…`. The page saves the token and removes it from the address bar. *Try elevation* there exercises the adult-PIN
flow.

## Day-to-day development
| Task | With `just` | Plain command |
|---|---|---|
| API and worker with auto-reload, plus the Vite dev server with hot reload | `just dev` | `cargo watch -x 'run -p fa-server -- serve all'` and `cd apps/web && corepack pnpm@9 dev` (needs `cargo install cargo-watch`) |
| Regenerate TS types, OpenAPI and the web client after API changes | `just gen` | see the `gen` recipe in [`justfile`](justfile) |
| Rust tests (uses the dev DB) | `just test` | `cargo test --workspace` |
| Lint | `just lint` | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` |
| Server CLI help | n/a | `cargo run -p fa-server -- --help` |

Vite (`corepack pnpm@9 dev`) serves the UI on its own port and proxies `/api` to `127.0.0.1:8080`.

**Database queries** are checked at compile time by sqlx. After adding or changing a `query!`,
regenerate the offline data with `cargo sqlx prepare --workspace` (install it with
`cargo install sqlx-cli --no-default-features --features rustls,postgres --locked`), and commit
`.sqlx/`. CI compiles with `SQLX_OFFLINE=true`.

**Migrations** live in `crates/fa-store/migrations/` and run automatically when the API starts.
Never edit a migration once it has been deployed anywhere. Add a new one instead.

## Checks: run CI locally
Run these before pushing:
```bash
just ci          # = scripts/ci-local.sh
just e2e         # = scripts/e2e.sh
```
| Script | What it does |
|---|---|
| [`scripts/ci-local.sh`](scripts/ci-local.sh) | Mirrors `.github/workflows/ci.yml`: fmt, clippy, tests against a **fresh throwaway Postgres** with `SQLX_OFFLINE=true`, cargo-deny (if installed), the example-data hygiene check, and web lint, typecheck, test and build. `--no-web` skips the web part |
| [`scripts/e2e.sh`](scripts/e2e.sh) | Throwaway Postgres, then `serve all`, then the Playwright API smoke test (`e2e/`) and UI smoke test (`apps/web/e2e/`) |
| [`scripts/compose-smoke.sh`](scripts/compose-smoke.sh) | Builds the image, starts `deploy/compose.example.yaml` and takes an on-demand backup |
| [`scripts/restore-rehearsal.sh`](scripts/restore-rehearsal.sh) | Restores a backup dump into a fresh Postgres and checks the app comes up with the same data |
| [`scripts/check-example-data.sh`](scripts/check-example-data.sh) | Fails if example data, fixtures or e2e files contain non-reserved email domains |

Always test against a **fresh** database. The long-lived dev DB holds state (roles, applied
migrations) that can hide failures CI will hit.

## Deploying
Deployment is up to whoever hosts it. We ship versioned images (`ghcr.io/<owner>/fa-server`) and
a sample [`deploy/compose.example.yaml`](deploy/compose.example.yaml) with profiles:
- **default:** Postgres, api, worker, ntfy and nightly backups;
- **`ai-cpu`:** small AI models on the core node (from M4);
- **`gpu`:** Ollama on a GPU machine.

Start with [`deploy/README.md`](deploy/README.md). It covers the master key (back it up offline),
Tailscale, two-machine and single-machine layouts, and first-run steps. Runbooks are in
[`deploy/runbooks/`](deploy/runbooks/), including install, restore and key rotation.

The app is meant to be reachable **only over Tailscale**. Never expose it to the public internet.

## Repository layout
```
crates/          Rust libraries: fa-domain (types and permissions), fa-store (DB, migrations, RLS,
                 encrypted secrets), fa-config, fa-auth, fa-jobs, fa-api
bins/            fa-server (server and operator CLI), fa-gpu-agent (GPU node helper, M4)
apps/web/        Vue 3 + TypeScript PWA, including the /dashboard kiosk view
e2e/             Playwright API smoke tests
deploy/          Compose sample, env example, systemd unit, runbooks
scripts/         Dev, CI and ops scripts
config/examples/ Fictional household config
arch/ design/ decisions/ research/ plan/ log/   Design system (see below)
tools/           Python helpers for the design docs (diagram generator, workflow helpers)
```

## How this project is designed
Design work is recorded in the repo, and each folder has an `_index.md`:
- [`PROJECT.md`](PROJECT.md): the original goals and requirements.
- [`arch/`](arch/_index.md): architecture docs, each paired with a generated UML draw.io diagram.
  Start with [`arch/Overall.md`](arch/Overall.md).
- [`design/`](design/_index.md): questions asked during design and their answers.
- [`decisions/`](decisions/_index.md): decision records (D-NNN) that come from those answers.
- [`research/`](research/_index.md): external research the decisions rely on.
- [`plan/`](plan/_index.md): milestone plans and task briefs.
- [`log/`](log/_index.md): a daily work log.

Key principles: it works without AI and AI makes it better; everything is configurable for any
household and hardware; it's local and private; and **no household data ever goes into this
repository**. Examples use `example.com`.

## Roadmap
| Milestone | Scope | Status |
|---|---|---|
| M0 | Skeleton: workspace, DB with RLS, config, auth and RBAC, devices and PIN, jobs, Vue shell, compose, backups, CI | ✅ code-complete |
| M1 | Members, weather, calendar read (Google, CalDAV, ICS), real dashboard | next |
| M2 | Tasks and chores (rotation, pools), ntfy notifications | |
| M3 | Gmail ingest, rules pipeline, bills, school and teacher linking, event approvals | |
| M4 | AI: System One decision models, local LLM, GPU node wake, eval harness | |
| M5 | Recipes, meal plan, grocery list | |
| M6 | Smart assignment, TUI | |

## License
[AGPL-3.0-or-later](LICENSE).
