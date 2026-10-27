---
id: T-09
title: "Docs: deploy guide, README, runbooks"
milestone: M3.5
wave: 5
depends_on: [T-02, T-03, T-04, T-05, T-06, T-07, T-08]
agent: {model: sonnet, effort: medium}
status: done
owns: ["README.md", "deploy/README.md", "deploy/runbooks/**", "config/examples/**"]
decisions: [D-065, D-072, D-073, D-074, D-075, D-076, D-077]
---

# T-09: Docs: deploy guide, README, runbooks

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md` (all), `arch/Platform.md` v4 §3, §6, §8, §9
- `deploy/compose.yaml`, `deploy/.env.example`, the reports of T-02 to T-08 (added below)
- CLAUDE.md, *README*: what must stay current

## You own
- `README.md`
- `deploy/README.md`
- `deploy/runbooks/**`
- `config/examples/**`

## Work
1. **`deploy/README.md` rewrite:**
   - **Install in three steps:** get Docker; download `compose.yaml` (raw GitHub URL of the release tag) or clone; `docker compose up -d`, then open `http://<server>:8080` and **finish setup right away** (D-073).
   - **Remote access:** the Tailscale profile (auth key, `TS_HOSTNAME`, phones on the tailnet, HTTPS address, ntfy on `:8443`).
   - **Advanced:** Tailscale on the host; your own reverse proxy (add it to trusted proxies in Settings, System); building locally (`compose.build.yaml`, also for arm64, D-077); the D-064 dashboard-only listener.
   - **Backups and the secrets volume:** what to copy off the machine, and the master-key backup from the wizard.
   - **Updates.**
   - **Troubleshooting.**
   - Calendars, mail and phones now point to the UI. Drop every `.env`-required step, the household TOML step and the `fa-server user add`/`pin set` steps (mention them under *Recovery* only).
2. **`README.md`:**
   - Deploying section;
   - Configuration reference: new env vars, `FA_DEFAULT_*`, which settings live in Settings, System, and that env locks them;
   - `fa-server` commands (`init`, `config import --dry-run`);
   - ports;
   - repository layout;
   - the M3.5 roadmap row.
   *Run it locally* keeps working (the dev flow may still use `config import`).
3. **Runbooks:**
   - `install.md`: point to the guide, or fold into it;
   - `restore.md`: the secrets volume, putting the master key back from its base64 backup;
   - `rotate-key.md`: new paths.
4. `config/examples/household.example.toml`: the header comment says it is optional and imported from Settings, System.
5. **Verify by running.** Bring up the guide's install with `-f deploy/compose.yaml -f deploy/compose.build.yaml` under project name `fa-docs`, on ports 18280 (app) and 18290 (ntfy) via `FA_HTTP_PORT`/`FA_NTFY_PORT`. Complete setup with `curl` against the setup API, follow the backup and restore steps, then `docker compose -p fa-docs down -v`. Fix the docs for anything that didn't work as written.

## Acceptance
- [ ] every command in `deploy/README.md` and the runbooks was run, or is marked as needing real hardware (Tailscale auth key, phones)
- [ ] no reference to `compose.example.yaml`, a required `.env` value, `FA_SETUP_CODE`, `lan_mode`, or arm64 images remains
- [ ] `python3 tools/fa.py check` clean; `docker ps -a --filter name=fa-docs` and `docker volume ls -f dangling=true` show nothing left from you

## Wave 1 outcomes: T-04 (deploy)
- Done. `init.rs`: `create_secrets(dir, owner)` (4 unit tests, run in a scratch crate because `fa-server` didn't compile during W1; reads `/dev/urandom` since `bins/fa-server/Cargo.toml` is T-07's). `Init` is dispatched before config loading.
- `deploy/compose.yaml`: `init`, `postgres`, `app`, `ntfy`, `backup`; profiles `tailscale` (`tailscale/tailscale:v1.104.2`, inline `configs: ts_serve`, so no `deploy/tailscale/` dir), `gpu`, `ai-cpu`. Network `fa` 172.30.89.0/24, sidecar .10. Needs **Compose ≥ 2.23.1** (`configs.content`). Verified by the main session: both profiles validate from the repo root with no `.env`.
- `deploy/compose.build.yaml` (build overlay), `deploy/.env.example` (all optional), `.github/workflows/release-image.yml` (amd64, `v*` tags).
- Open: `backup` uses `POSTGRES_PASSWORD_FILE` (documented by the image, untested); `docker compose up` without the build overlay needs H-12's first published image.
- Document: Compose ≥ 2.23.1; the single `compose.yaml` with no required `.env`; profiles `tailscale`, `gpu`, `ai-cpu`; the build overlay (`-f compose.yaml -f compose.build.yaml up -d --build`), which is the only way to start before H-12 publishes an image; `fa-server init [--secrets-dir]`; the variables in `deploy/.env.example`.

## Wave 2 outcomes: T-03 (fa-config)
- Done; `cargo test -p fa-config` green on a fresh DB. Exports: `fa_config::{runtime, CookieSecure, RUNTIME_KEYS, RuntimeKey, RuntimeSnapshot, validate_value, create_household}`. `RuntimeSnapshot::{views(), source(key), is_locked(key)}`; `ProcessConfig::load_layered(&BTreeMap<String, Value>)` (plus a testable `load_layered_from`); `load()` = `load_layered` with an empty map.
- `public_base_url: Option<String>` → use `effective_base_url()`; `cookie_secure: CookieSecure`; new fields as in the contract. `public_url` (`FA_PUBLIC_URL`) still exists, with no required check.
- **Empty env vars count as unset** (they neither lock nor hide the DB value): `FA_TRUSTED_PROXIES=""` no longer means "trust none"; an empty list comes from the DB. A key set in `fa.toml` counts as `env`/locked.
- `load_layered` expects **decrypted** secrets (`ntfy_token`, `google_client_secret`) as plain strings in the map; an invalid DB value is skipped with a warning naming only the key.
- `create_household` returns `ImportError`; `import(store, hid, cfg, mode, dry_run)`; `ImportReport: ToSchema`.
- README: `FA_DATABASE_PASSWORD_FILE`, `FA_TAILNET_BASE_URL`, `FA_ALLOW_PRIVATE_HOSTS`, the `FA_DEFAULT_<KEY>` family, `FA_COOKIE_SECURE=auto|true|false` (default `auto`), `FA_PUBLIC_BASE_URL` optional, an empty env var counts as unset, a key in `fa.toml` locks like env.

## Wave 2 outcomes: T-05 (feature crates)
- Done. The main session ran all compiling crates together on a fresh DB: 603 passed, 0 failed; clippy and fmt clean.
- `InboxService`: `counts(household, viewer)`; `approve_bill/reject_bill/approve_contact/reject_contact` take `viewer: Viewer`; any non-adult viewer (devices too) gets `MailError::Forbidden`. New public guards **`InboxService::{ensure_message, ensure_event, ensure_task}`**: event approve/edit/reject and suggested-task accept/dismiss live in fa-calendar/fa-tasks, so the **API must call these first**.
- `Http::with_policy(allow_http, allow_private_hosts)`, `ConnectorError::PrivateHost` (→ `CalendarError::Invalid`). `ConnectorConfig`, `WeatherConfig`, `MailConfig` have `allow_private_hosts` (default false). `MailDeps::http` is built by the server. IMAP (raw `TcpStream`) is **not** guarded; C-05 covers HTTP only (known gap).
- Advisory clearing after each successful evaluation; nothing is cleared if any active rule fails to parse.
- `approval_needed` goes only to the source's adults (two mail tests now expect one row).
- Document `allow_private_hosts` / `FA_ALLOW_PRIVATE_HOSTS` (default false): turn it on for a LAN CalDAV server or other feeds on the home network; IMAP is not affected.

## Wave 3 outcomes: T-06 (fa-auth + fa-api)
- Done. The main session ran `cargo test --workspace --exclude fa-server` on a fresh DB: 694 passed, 0 failed; clippy and fmt clean. `apps/web/src/gen/openapi.json` and `api.d.ts` regenerated (97 paths) with `cargo run -p fa-api --example openapi -- <out>` (same output as `fa-server gen-openapi`, usable while fa-server doesn't compile).
- Errors: `Problem` has an optional `code` (`setup_required`, `setup_closed`, `setting_locked`, `invalid_setting` with `field` = the key).
- `GET`/`PATCH /household` return `HouseholdViewExt` = `HouseholdView` flattened + `dashboard_pending_approvals` (OpenAPI only, no ts-rs type).
- `POST /members/{id}/account/password` takes `PasswordBody` (untagged): **yourself** → `OwnPasswordChange {current_password, new_password}` (a bare `{password}` for yourself is 422); an adult resetting a child still sends `{password}`.
- Inbox routes are adult-user-only (devices 403). `DashboardView.pending_approvals` replaces `inbox`. The dismiss route is gone. While setup is needed, every `/api/v1/*` except `/setup*` (including `/openapi.json`) answers 409 `setup_required`.
- `POST /system/master-key/confirm` writes `key_backup_confirmed_at` through `InstanceSettingRepo::set`, which fires `fa_settings`.
- Setup/household and config import seed the default weather rules (`fa_weather::ensure_default_rules`).
- README: default trusted proxies are loopback only (compose sets `FA_DEFAULT_TRUSTED_PROXIES`); `FA_COOKIE_SECURE` auto follows `X-Forwarded-Proto: https` from a trusted proxy; new routes `/setup*`, `/system/*`, `/config/*`; the own-password API needs the current password; advisory dismiss and the dashboard inbox are gone.

## Wave 4 outcomes: T-07 (server) and main-session integration
- Done. The main session ran `cargo test --workspace` on a fresh DB: **705 passed, 0 failed**; clippy and fmt clean; `cargo sqlx prepare --workspace` leaves `.sqlx` unchanged.
- `serve` resolves settings in a loop: on `fa_settings` it restarts in about a second, **only if the resolved runtime values changed** (`master-key/confirm` doesn't restart). If a pass fails to build or bind, it falls back to env-only config. Log line: `settings changed, restarting`. Test: `bins/fa-server/tests/settings_restart.rs`, which spawns the real binary.
- `fa-server config import --dry-run`. **A CLI import that creates the household now marks setup complete** (main session, with a test), so CLI-seeded households (e2e, scripted installs) skip the wizard. A dry run writes nothing.
- `device pair` uses the stored settings (`serve::layered_config`), so pairing links use the address that setup saved.
- Link base for OAuth and notifications: `FA_PUBLIC_URL`, else `tailnet_base_url`, else `effective_base_url()`.
- `init.rs` still reads `/dev/urandom` (fine, Linux only).
- Document `config import --dry-run`, that changing a setting in Settings, System restarts the app by itself in about a second, and `FA_ALLOW_PRIVATE_HOSTS=true` for dev (local stubs). *Run it locally* still seeds with `config import` (it now marks setup complete).
