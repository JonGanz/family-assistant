---
id: T-07
title: "fa-server: settings loop with restart, wiring, ops"
milestone: M3.5
wave: 4
depends_on: [T-03, T-05, T-06]
agent: {model: sonnet, effort: medium}
status: done
owns: ["bins/fa-server/src/serve.rs", "bins/fa-server/src/wiring.rs", "bins/fa-server/src/ops.rs", "bins/fa-server/src/mail.rs", "bins/fa-server/Cargo.toml", "bins/fa-server/tests/**"]
decisions: [D-065, D-069, D-075, D-076]
---

# T-07: fa-server: settings loop with restart, wiring, ops

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: *Contracts fixed up front* → Server, fa-config, Feature crates, API (`ApiConfig`)
- `arch/Platform.md` §3, §6
- `bins/fa-server/src/*`, `crates/fa-api/src/bridge.rs` (the existing PgListener pattern)
- T-02 to T-06 outcomes (added below)

## You own
- `bins/fa-server/src/serve.rs`
- `bins/fa-server/src/wiring.rs`
- `bins/fa-server/src/ops.rs`
- `bins/fa-server/src/mail.rs`
- `bins/fa-server/Cargo.toml`
- `bins/fa-server/tests/**`

## Work
1. **Settings loop:** `serve` does the following, in order:
   1. connect;
   2. run migrations if this process runs the api (as today);
   3. loop: read `InstanceSettingRepo::all` and decrypt the secret keys with the master key, `ProcessConfig::load_layered`, build the services and `ApiConfig` (`cookie_secure`, `runtime`, `master_key_path`, `trusted_proxies` parsed from the config, and the LAN listener from the config instead of raw env), then run until SIGTERM (exit) or a `fa_settings` notification (cancel, wait for the tasks to finish, log `settings changed, restarting`, next pass).
   Debounce notifications by 500 ms. A pass that fails to build (a bad DB value) logs the error, falls back to the env-only config, and keeps serving rather than crash-looping.
2. **Wiring:**
   - `public_base_url` goes through `effective_base_url()` (`AuthConfig`, `NotifyConfig`, the OAuth redirect: `FA_PUBLIC_URL` still wins; else `tailnet_base_url` when set; else the effective base).
   - `allow_private_hosts` goes into `ConnectorConfig`, `WeatherConfig` and `MailConfig`.
   - The OAuth and mail `Http` clients use `Http::with_policy`.
   - Remove the raw `std::env::var` reads that fa-config now covers.
3. **Ops:**
   - Every CLI command keeps working.
   - `config import` gains `--dry-run`.
   - The "no household yet" message says to open the app in a browser to run setup, or to use `config import`.
4. Delete code made dead by the changes. `serve api` and `serve worker` stay (D-076: not documented for deployment).

## Acceptance
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes, `cargo clippy --workspace --all-targets -- -D warnings` clean, `cargo fmt --check` clean
- [ ] `cargo test --workspace` passes on a fresh container (port 55407)
- [ ] a test (in `bins/fa-server/tests/`, against a fresh container) starts `serve all` in-process on a free port, PATCHes a runtime setting through the API, and observes the restart (`/readyz` comes back and `GET /system/settings` shows the new value with source `db`)
- [ ] `fa-server gen-openapi --out <tmp>` succeeds

## Wave 1 outcomes: T-04 (deploy)
- Done. `init.rs`: `create_secrets(dir, owner)` (4 unit tests, run in a scratch crate because `fa-server` didn't compile during W1; reads `/dev/urandom` since `bins/fa-server/Cargo.toml` is T-07's). `Init` is dispatched before config loading.
- `deploy/compose.yaml`: `init`, `postgres`, `app`, `ntfy`, `backup`; profiles `tailscale` (`tailscale/tailscale:v1.104.2`, inline `configs: ts_serve`, so no `deploy/tailscale/` dir), `gpu`, `ai-cpu`. Network `fa` 172.30.89.0/24, sidecar .10. Needs **Compose ≥ 2.23.1** (`configs.content`). Verified by the main session: both profiles validate from the repo root with no `.env`.
- `deploy/compose.build.yaml` (build overlay), `deploy/.env.example` (all optional), `.github/workflows/release-image.yml` (amd64, `v*` tags).
- Open: `backup` uses `POSTGRES_PASSWORD_FILE` (documented by the image, untested); `docker compose up` without the build overlay needs H-12's first published image.
- Run `init`'s unit tests (`cargo test -p fa-server init`) now that the crate compiles; optionally switch `init.rs` to `rand` (you own `Cargo.toml`).

## Wave 1 outcomes: T-02 (domain + store), read before starting
- Done; fa-domain 262 tests, fa-store all binaries green on a fresh DB. The main session re-ran fa-domain, fa-store, fa-pipeline, fa-notify and fa-tasks together: 383 passed, 0 failed.
- New: `fa_domain::system_api` (every contract type; `serde_json::Value` fields use `#[ts(type = "unknown")]`), `fa_store::{InstanceSettingRepo, SetupRepo}` (files `repo/instance_setting.rs`, `repo/setup.rs`), `AdvisoryRepo::retain_rule_advisories`, `NotificationRepo::insert_for_source_adults`, `InboxRepo::pending_by_adult` (one row per active adult, zeros included).
- Removed: `Action::DismissAdvisory`, `AdvisoryRepo::dismiss`, the advisory dismissed fields, `DashboardView.inbox`. `ApproveEvent` and `ReadInbox` now need an adult user (devices refused, elevated or not).
- `Household` gained `dashboard_pending_approvals: bool` and `setup_completed_at: Option<DateTime<Utc>>`; every struct literal must set them (tests: `true`, `None`).
- `pin_lockout` / `UserAccountRepo::lockout` read `FOR UPDATE`; `record_pin_failure` / `record_failure` don't update a currently locked row.
- After the workspace compiles, run `cargo sqlx prepare --workspace` and confirm `.sqlx` is unchanged (T-02 regenerated the fa-store entries by hand).

## Wave 2 outcomes: T-03 (fa-config)
- Done; `cargo test -p fa-config` green on a fresh DB. Exports: `fa_config::{runtime, CookieSecure, RUNTIME_KEYS, RuntimeKey, RuntimeSnapshot, validate_value, create_household}`. `RuntimeSnapshot::{views(), source(key), is_locked(key)}`; `ProcessConfig::load_layered(&BTreeMap<String, Value>)` (plus a testable `load_layered_from`); `load()` = `load_layered` with an empty map.
- `public_base_url: Option<String>` → use `effective_base_url()`; `cookie_secure: CookieSecure`; new fields as in the contract. `public_url` (`FA_PUBLIC_URL`) still exists, with no required check.
- **Empty env vars count as unset** (they neither lock nor hide the DB value): `FA_TRUSTED_PROXIES=""` no longer means "trust none"; an empty list comes from the DB. A key set in `fa.toml` counts as `env`/locked.
- `load_layered` expects **decrypted** secrets (`ntfy_token`, `google_client_secret`) as plain strings in the map; an invalid DB value is skipped with a warning naming only the key.
- `create_household` returns `ImportError`; `import(store, hid, cfg, mode, dry_run)`; `ImportReport: ToSchema`.
- **Your breakage:** `bins/fa-server/src/ops.rs` ~104 and ~524 call `import` without `dry_run`; `serve.rs` raw env reads (`trusted_proxies`, `lan_config`, `lan_base_url`, `env_flag`) move to `ProcessConfig` fields; every `cfg.public_base_url` reader uses `effective_base_url()`; `cookie_secure` is an enum. Decrypt secret settings (`InstanceSettingRepo::secret`) before `load_layered`.

## Wave 2 outcomes: T-05 (feature crates)
- Done. The main session ran all compiling crates together on a fresh DB: 603 passed, 0 failed; clippy and fmt clean.
- `InboxService`: `counts(household, viewer)`; `approve_bill/reject_bill/approve_contact/reject_contact` take `viewer: Viewer`; any non-adult viewer (devices too) gets `MailError::Forbidden`. New public guards **`InboxService::{ensure_message, ensure_event, ensure_task}`**: event approve/edit/reject and suggested-task accept/dismiss live in fa-calendar/fa-tasks, so the **API must call these first**.
- `Http::with_policy(allow_http, allow_private_hosts)`, `ConnectorError::PrivateHost` (→ `CalendarError::Invalid`). `ConnectorConfig`, `WeatherConfig`, `MailConfig` have `allow_private_hosts` (default false). `MailDeps::http` is built by the server. IMAP (raw `TcpStream`) is **not** guarded; C-05 covers HTTP only (known gap).
- Advisory clearing after each successful evaluation; nothing is cleared if any active rule fails to parse.
- `approval_needed` goes only to the source's adults (two mail tests now expect one row).
- **Your breakage:** `wiring.rs` ~80 and ~97 build `Http::new(cfg.allow_http_feeds ..)`: use `Http::with_policy(.., cfg.allow_private_hosts)` for OAuth and mail; set `allow_private_hosts` in `ConnectorConfig`, `WeatherConfig` and the `MailConfig` literal (~83). **Dev and e2e use local stubs**: `.env.dev` is not yours, so report the needed `FA_ALLOW_PRIVATE_HOSTS=true` for T-09/T-10.

## Wave 3 outcomes: T-06 (fa-auth + fa-api)
- Done. The main session ran `cargo test --workspace --exclude fa-server` on a fresh DB: 694 passed, 0 failed; clippy and fmt clean. `apps/web/src/gen/openapi.json` and `api.d.ts` regenerated (97 paths) with `cargo run -p fa-api --example openapi -- <out>` (same output as `fa-server gen-openapi`, usable while fa-server doesn't compile).
- Errors: `Problem` has an optional `code` (`setup_required`, `setup_closed`, `setting_locked`, `invalid_setting` with `field` = the key).
- `GET`/`PATCH /household` return `HouseholdViewExt` = `HouseholdView` flattened + `dashboard_pending_approvals` (OpenAPI only, no ts-rs type).
- `POST /members/{id}/account/password` takes `PasswordBody` (untagged): **yourself** → `OwnPasswordChange {current_password, new_password}` (a bare `{password}` for yourself is 422); an adult resetting a child still sends `{password}`.
- Inbox routes are adult-user-only (devices 403). `DashboardView.pending_approvals` replaces `inbox`. The dismiss route is gone. While setup is needed, every `/api/v1/*` except `/setup*` (including `/openapi.json`) answers 409 `setup_required`.
- `POST /system/master-key/confirm` writes `key_backup_confirmed_at` through `InstanceSettingRepo::set`, which fires `fa_settings`.
- Setup/household and config import seed the default weather rules (`fa_weather::ensure_default_rules`).
- **`ApiConfig`** fields: `cookie_secure: fa_config::CookieSecure`; `runtime: Arc<fa_config::RuntimeSnapshot>` (second value of `load_layered`); `master_key_path: PathBuf`; `tailnet_base_url: Option<String>`; `lan_base_url`; `trusted_proxies: Vec<IpNet>` parsed from the config CIDRs (`"…".parse::<IpNet>()`); empty `lan_allowed_cidrs` → `LanConfig::default_allowed()`.
- **`AuthConfig`**: `public_base_url = effective_base_url()`; optional `device_pin_burst` (6) and `device_pin_refill` (180 s). The device limiter lives in `AuthService`: rebuilt with the service each pass (acceptable).
- **Restart only on a real change:** on `fa_settings`, re-read and re-resolve; restart only if the resolved runtime values (secrets compared by value, not shown) differ from the running pass. `key_backup_confirmed_at` is not a runtime key, so confirming the backup must not restart. Test this.
