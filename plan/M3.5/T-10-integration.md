---
id: T-10
title: "Integration: e2e, compose smoke, CI"
milestone: M3.5
wave: 5
depends_on: [T-02, T-03, T-04, T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: high}
status: done
owns: [".env.dev", "e2e/**", "apps/web/e2e/**", "scripts/**", ".github/workflows/ci.yml", "deploy/compose.example.yaml", "deploy/compose.dev.yaml"]
decisions: [D-065, D-068, D-069, D-072, D-073, D-074, D-078]
---

# T-10: Integration: e2e, compose smoke, CI

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md` *Done means* (your checklist), all contracts
- `scripts/{e2e,compose-smoke,ci-local}.sh`, `e2e/`, `apps/web/e2e/`, `.github/workflows/ci.yml`
- the reports of T-02 to T-08 (added below)

## You own
- `e2e/**`
- `apps/web/e2e/**`
- `scripts/**`
- `.github/workflows/ci.yml`
- `deploy/compose.example.yaml`
- `deploy/compose.dev.yaml`

## Work
1. **e2e (API, `e2e/`):**
   - `setup.spec.ts` against an api instance on an **empty** database. Add a second instance to `scripts/e2e.sh` with its own DB and port 18094; the main suite keeps its CLI-seeded household. It covers the full wizard API, 409s after completion, and `setup_required` before it.
   - Cookie behaviour: no `Secure` over plain HTTP; `Secure` with `X-Forwarded-Proto: https` from loopback.
   - Removed routes 404; the device inbox gets 403; adult-scoped approvals (two adults, an owners-only source and a household source); the dashboard `pending_approvals`.
   - Advisories clear when the weather stub stops matching.
   - Settings PATCH → restart → new value.
2. **UI e2e (`apps/web/e2e/`):**
   - the wizard end to end on the empty instance (plain HTTP origin, which also proves the insecure-context path);
   - Settings, System;
   - the dashboard has no inbox and no dismiss, and shows the members badge;
   - a child can't reach `/settings/members`, and `/members` is gone.
3. **`scripts/compose-smoke.sh`:** rewrite for `deploy/compose.yaml` + `deploy/compose.build.yaml`, **with no `.env`**:
   - project `fa-smoke`, ports 18180/18190;
   - check `init` (file modes and owners, idempotent on a second `up`);
   - `/readyz`, then setup through the API;
   - login cookie without `Secure`;
   - `docker compose --profile tailscale config` validates;
   - finish with `down -v`.
   Remove `deploy/compose.example.yaml` and every script and CI reference to it.
4. **CI:**
   - `ci.yml` runs the new smoke where it ran the old one;
   - the `docs` job already runs `python3 tools/fa.py check-yaml .github/workflows/*.yml deploy/*.yaml` (T-01); keep it passing;
   - `scripts/ci-local.sh` matches.
5. Throwaway containers: `docker rm -f -v`. Leave 0 dangling volumes.
6. **Time-of-day robustness:** `e2e/mail-send.ts` renders `{{d+N}}` in UTC; render it in the household time zone (America/New_York), as T-01 did for `e2e/stubs/server.mjs` and `feeds.spec.ts`. Search the specs for other UTC-date assumptions.

## Acceptance
- [ ] `scripts/ci-local.sh`, `scripts/e2e.sh` and `scripts/compose-smoke.sh` pass; quote the spec and test counts from their output
- [ ] `docker volume ls -f dangling=true` is empty afterwards
- [ ] the report lists every *Done means* item and how it was verified, or why not (real-hardware steps belong to the shakedown)

## Wave 1 outcomes: T-04 (deploy)
- Done. `init.rs`: `create_secrets(dir, owner)` (4 unit tests, run in a scratch crate because `fa-server` didn't compile during W1; reads `/dev/urandom` since `bins/fa-server/Cargo.toml` is T-07's). `Init` is dispatched before config loading.
- `deploy/compose.yaml`: `init`, `postgres`, `app`, `ntfy`, `backup`; profiles `tailscale` (`tailscale/tailscale:v1.104.2`, inline `configs: ts_serve`, so no `deploy/tailscale/` dir), `gpu`, `ai-cpu`. Network `fa` 172.30.89.0/24, sidecar .10. Needs **Compose ≥ 2.23.1** (`configs.content`). Verified by the main session: both profiles validate from the repo root with no `.env`.
- `deploy/compose.build.yaml` (build overlay), `deploy/.env.example` (all optional), `.github/workflows/release-image.yml` (amd64, `v*` tags).
- Open: `backup` uses `POSTGRES_PASSWORD_FILE` (documented by the image, untested); `docker compose up` without the build overlay needs H-12's first published image.
- Verify in real containers: `init` (modes 0400/0444, owner 10001 on `master_key`, idempotent), and that the `backup` service authenticates with `POSTGRES_PASSWORD_FILE` and writes a dump.

## Wave 2 outcomes: T-05 (feature crates)
- Done. The main session ran all compiling crates together on a fresh DB: 603 passed, 0 failed; clippy and fmt clean.
- `InboxService`: `counts(household, viewer)`; `approve_bill/reject_bill/approve_contact/reject_contact` take `viewer: Viewer`; any non-adult viewer (devices too) gets `MailError::Forbidden`. New public guards **`InboxService::{ensure_message, ensure_event, ensure_task}`**: event approve/edit/reject and suggested-task accept/dismiss live in fa-calendar/fa-tasks, so the **API must call these first**.
- `Http::with_policy(allow_http, allow_private_hosts)`, `ConnectorError::PrivateHost` (→ `CalendarError::Invalid`). `ConnectorConfig`, `WeatherConfig`, `MailConfig` have `allow_private_hosts` (default false). `MailDeps::http` is built by the server. IMAP (raw `TcpStream`) is **not** guarded; C-05 covers HTTP only (known gap).
- Advisory clearing after each successful evaluation; nothing is cleared if any active rule fails to parse.
- `approval_needed` goes only to the source's adults (two mail tests now expect one row).
- **`FA_ALLOW_PRIVATE_HOSTS=true`** must be set for the e2e stack (ICS fixture server, Open-Meteo stub, Radicale and the Gmail stub are all on 127.0.0.1) and in `.env.dev` (you own `scripts/**`; ask the main session for `.env.dev` if it's not yours). Add an e2e check that, with it unset, a calendar source at a private address is refused with a clear error.

## Wave 3 outcomes: T-06 (fa-auth + fa-api)
- Done. The main session ran `cargo test --workspace --exclude fa-server` on a fresh DB: 694 passed, 0 failed; clippy and fmt clean. `apps/web/src/gen/openapi.json` and `api.d.ts` regenerated (97 paths) with `cargo run -p fa-api --example openapi -- <out>` (same output as `fa-server gen-openapi`, usable while fa-server doesn't compile).
- Errors: `Problem` has an optional `code` (`setup_required`, `setup_closed`, `setting_locked`, `invalid_setting` with `field` = the key).
- `GET`/`PATCH /household` return `HouseholdViewExt` = `HouseholdView` flattened + `dashboard_pending_approvals` (OpenAPI only, no ts-rs type).
- `POST /members/{id}/account/password` takes `PasswordBody` (untagged): **yourself** → `OwnPasswordChange {current_password, new_password}` (a bare `{password}` for yourself is 422); an adult resetting a child still sends `{password}`.
- Inbox routes are adult-user-only (devices 403). `DashboardView.pending_approvals` replaces `inbox`. The dismiss route is gone. While setup is needed, every `/api/v1/*` except `/setup*` (including `/openapi.json`) answers 409 `setup_required`.
- `POST /system/master-key/confirm` writes `key_backup_confirmed_at` through `InstanceSettingRepo::set`, which fires `fa_settings`.
- Setup/household and config import seed the default weather rules (`fa_weather::ensure_default_rules`).
- e2e: cover the problem `code`s, `PasswordBody` (own change needs the current password; adult reset of a child doesn't), devices 403 on inbox, `/openapi.json` 409 before setup.

## Wave 4 outcomes: T-08 (web)
- Done; `apps/web` format, lint, typecheck, 252 vitest tests and build pass. Not run in a browser: the UI e2e is the first real check.
- **UI e2e specs to update:**
  - `smoke.spec.ts`: the top-nav "Members" link is gone (Members is at `/settings/members`, `ul.members li` is unchanged); the header "Settings" link now opens the Settings index at `/settings`; the "Settings" sub-navigation exists only inside `/settings/*`.
  - `m3.spec.ts` ~191-245, "Dashboard inbox panel …": obsolete (the `inbox` panel and `inbox-total`/`inbox-counts`/`inbox-locked` are removed). Replace it with the members-strip badge `data-testid="approvals-<memberId>"`.
  - The own password needs `{current_password, new_password}`.
  - Over plain HTTP from a non-localhost origin, Google shows a "needs HTTPS" note instead of the Google tab.
  - Tab roles and names are unchanged ("Events (n)").
- **Setup redirect:** an adult in a household with `setup_completed_at` NULL is sent to `/setup`. The main session makes a CLI `config import` that *creates* the household mark setup complete (after T-07); if that's not yet in, seed with `POST /api/v1/setup/complete`.
- The wizard stores its step in `localStorage` `fa.setup.step`; the service worker is registered by hand (`/sw.js`) only in a secure context.

## Wave 4 outcomes: T-07 (server) and main-session integration
- Done. The main session ran `cargo test --workspace` on a fresh DB: **705 passed, 0 failed**; clippy and fmt clean; `cargo sqlx prepare --workspace` leaves `.sqlx` unchanged.
- `serve` resolves settings in a loop: on `fa_settings` it restarts in about a second, **only if the resolved runtime values changed** (`master-key/confirm` doesn't restart). If a pass fails to build or bind, it falls back to env-only config. Log line: `settings changed, restarting`. Test: `bins/fa-server/tests/settings_restart.rs`, which spawns the real binary.
- `fa-server config import --dry-run`. **A CLI import that creates the household now marks setup complete** (main session, with a test), so CLI-seeded households (e2e, scripted installs) skip the wizard. A dry run writes nothing.
- `device pair` uses the stored settings (`serve::layered_config`), so pairing links use the address that setup saved.
- Link base for OAuth and notifications: `FA_PUBLIC_URL`, else `tailnet_base_url`, else `effective_base_url()`.
- `init.rs` still reads `/dev/urandom` (fine, Linux only).
- You also own **`.env.dev`**: add `FA_ALLOW_PRIVATE_HOSTS=true` there and to the e2e env. Rerun `bins/fa-server/tests/settings_restart.rs` as part of the suites (it needs `DATABASE_URL`).

## Wave 5 outcomes
Done. New specs: `e2e/setup.spec.ts`, `e2e/approvals.spec.ts`, `apps/web/e2e/wizard.spec.ts`, `apps/web/e2e/m35.spec.ts`. `compose-smoke.sh` was rewritten for `deploy/compose.yaml` with no `.env` and added to CI; `deploy/compose.example.yaml` was removed. The date helpers now use the America/New_York calendar day (DST-safe). Follow-ups it raised were fixed by the main session (weather `today` window, CLI settings layering, read retry around settings restarts) or listed as known gaps in `plan/M3.5.md`.
