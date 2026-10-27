---
id: T-06
title: "fa-auth + fa-api: setup, system and config routes, Secure per request, removals, review fixes"
milestone: M3.5
wave: 3
depends_on: [T-02, T-03]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-auth/**", "crates/fa-api/**"]
decisions: [D-065, D-068, D-069, D-070, D-072, D-073, D-074, D-075, D-078]
---

# T-06: fa-auth + fa-api: setup, system and config routes, Secure per request, removals, review fixes

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: *Contracts fixed up front* → API, fa-domain, fa-store, fa-config, Feature crates (the `InboxService` signature)
- `arch/Platform.md` §3, §5, §6; `arch/Frontend.md` §2a (setup flow), §3c (System)
- `crates/fa-api/src/{lib,routes,extract,lan,feeds,mail,openapi}.rs`, `crates/fa-auth/src/{login,pin,limiter,accounts,config,devices,service}.rs`
- IMPROVEMENTS.md C-01b, C-03, C-04, C-12, C-15 (file and line references)
- T-02, T-03 and T-05 outcomes (added below)

## You own
- `crates/fa-auth/**`
- `crates/fa-api/**`

## Work
1. **Setup (D-073):** the routes in the contract table, plus the setup-required layer.
   - `POST /setup/household` calls `fa_config::create_household` and stores `base_url` through `InstanceSettingRepo` (as `tailnet_base_url` for an https `*.ts.net` origin, else `public_base_url`), then `notify_changed`.
   - `POST /setup/owner` creates the adult member, the account (password rules unchanged, D-031), the optional PIN and the session (cookie as at login) in one transaction.
   - Serialize both under `pg_advisory_xact_lock`, so two concurrent visitors can't both win. The loser gets 409 `setup_closed`.
   - `SetupStatus.secure`, `origin` and `via_tailscale` come from the request (see 3).
2. **System:** `POST /system/master-key` verifies the adult's password with the same limiter and lockout as login, reads `ApiConfig.master_key_path`, returns base64, and writes an audit entry (never the key). Then `confirm`, and `GET`/`PATCH /system/settings`:
   - validate each value with `fa_config::runtime::validate_value`;
   - refuse locked keys;
   - write values and secrets with `InstanceSettingRepo`, plus `notify_changed`, in one transaction;
   - answer with the `SystemSettingsView` built from `ApiConfig.runtime` plus the pending DB values.
   The server applies the change on its restart (T-07).
3. **Request scheme:**
   - A helper `request_is_secure(&Request, &ApiConfig) -> bool`: TLS is never terminated here, so a request counts as secure only when `X-Forwarded-Proto: https` comes from a trusted proxy.
   - `origin` comes from `Host` (or `X-Forwarded-Host` from a trusted proxy).
   - Build the cookie with `axum_extra::extract::cookie::Cookie` (C-15; `axum-extra` already has the `cookie` feature, so no new dependency): `HttpOnly`, `SameSite=Lax`, and `Secure` per `CookieSecure` (`Auto` follows the helper).
   - No HSTS anywhere.
   - `ApiConfig.trusted_proxies` defaults to loopback only (C-04).
4. **Config routes:** import with `dry_run` (1 MiB body limit, `text/plain`) and export as an attachment.
5. **Removals (D-068):**
   - the dismiss route;
   - the dashboard `inbox` field;
   - the D-064 allowlist entries for `inbox`, approve, reject and dismiss.
   The dashboard handler fills `pending_approvals` from `InboxRepo::pending_by_adult` when `dashboard_pending_approvals` is on. Inbox routes pass the viewer to the scoped `InboxService`. `HouseholdPatch.dashboard_pending_approvals` is handled.
6. **Members (D-070):** member writes are adult-only through `Policy`. Check every member route.
7. **Review fixes:**
   - C-03: the own-password route takes `OwnPasswordChange` and revokes the user's other sessions.
   - C-12: argon2 verify runs outside any open transaction (read, release, verify, short transaction to record) in login and PIN.
   - C-01b: the device limiter budget moves from a process-wide `OnceLock` into `AuthConfig`.
8. **OpenAPI:** register every new and changed path and schema. `fa-server gen-openapi` must succeed after T-07 (the main session runs it).
9. **Pairing links:** per the contract (`tailnet_base_url` on secure requests, else `effective_base_url()`; the LAN link as today).

## Acceptance
- [ ] `cargo test -p fa-auth -p fa-api` pass (fresh container port 55406)
- [ ] new tests:
  - setup happy path; a second household or owner gets 409; concurrent owners give exactly one winner; 409 `setup_required` on other routes before setup;
  - the Secure cookie is set only with `X-Forwarded-Proto: https` from a trusted peer, and that header from an untrusted peer is ignored;
  - the master key needs the password, is audited, and a wrong password counts toward lockout;
  - settings: a locked key gives 409; an invalid value gives 422 naming the key; a secret is never echoed;
  - import dry run writes nothing;
  - removed routes 404 on both listeners; a device gets 403 on the inbox;
  - an adult can't approve another adult's owners-only item;
  - `pending_approvals` respects the household switch;
  - C-03: a wrong current password gives 401, and success revokes the other sessions
- [ ] `cargo clippy -p fa-auth -p fa-api --all-targets -- -D warnings` and `cargo fmt --check` clean

## Wave 1 outcomes: T-02 (domain + store), read before starting
- Done; fa-domain 262 tests, fa-store all binaries green on a fresh DB. The main session re-ran fa-domain, fa-store, fa-pipeline, fa-notify and fa-tasks together: 383 passed, 0 failed.
- New: `fa_domain::system_api` (every contract type; `serde_json::Value` fields use `#[ts(type = "unknown")]`), `fa_store::{InstanceSettingRepo, SetupRepo}` (files `repo/instance_setting.rs`, `repo/setup.rs`), `AdvisoryRepo::retain_rule_advisories`, `NotificationRepo::insert_for_source_adults`, `InboxRepo::pending_by_adult` (one row per active adult, zeros included).
- Removed: `Action::DismissAdvisory`, `AdvisoryRepo::dismiss`, the advisory dismissed fields, `DashboardView.inbox`. `ApproveEvent` and `ReadInbox` now need an adult user (devices refused, elevated or not).
- `Household` gained `dashboard_pending_approvals: bool` and `setup_completed_at: Option<DateTime<Utc>>`; every struct literal must set them (tests: `true`, `None`).
- `pin_lockout` / `UserAccountRepo::lockout` read `FOR UPDATE`; `record_pin_failure` / `record_failure` don't update a currently locked row.
- **Your breakage to fix:** `crates/fa-api/src/feeds.rs` (`Action::DismissAdvisory` ~1179, `AdvisoryRepo::dismiss` ~1182, `DashboardView { inbox }` ~1653) and Household literals in `crates/fa-auth/tests/{auth.rs,oauth.rs}`. **`pin_lockout`/`lockout` must be read in the same transaction as `record_*`** so `FOR UPDATE` serialises attempts (`fa-auth/src/pin.rs` ~91 and the login path); with C-12, keep that: read+lock, release nothing until recorded, or re-read under lock after the verify. The device limiter in `pin.rs` was changed by the main session during the baseline (burst 6, refund on success); move it into `AuthConfig` (C-01b) keeping that behaviour.

## Wave 2 outcomes: T-03 (fa-config)
- Done; `cargo test -p fa-config` green on a fresh DB. Exports: `fa_config::{runtime, CookieSecure, RUNTIME_KEYS, RuntimeKey, RuntimeSnapshot, validate_value, create_household}`. `RuntimeSnapshot::{views(), source(key), is_locked(key)}`; `ProcessConfig::load_layered(&BTreeMap<String, Value>)` (plus a testable `load_layered_from`); `load()` = `load_layered` with an empty map.
- `public_base_url: Option<String>` → use `effective_base_url()`; `cookie_secure: CookieSecure`; new fields as in the contract. `public_url` (`FA_PUBLIC_URL`) still exists, with no required check.
- **Empty env vars count as unset** (they neither lock nor hide the DB value): `FA_TRUSTED_PROXIES=""` no longer means "trust none"; an empty list comes from the DB. A key set in `fa.toml` counts as `env`/locked.
- `load_layered` expects **decrypted** secrets (`ntfy_token`, `google_client_secret`) as plain strings in the map; an invalid DB value is skipped with a warning naming only the key.
- `create_household` returns `ImportError`; `import(store, hid, cfg, mode, dry_run)`; `ImportReport: ToSchema`.
- An empty `lan_allowed_cidrs` means private and loopback: map it to `LanConfig::default_allowed()`.

## Wave 2 outcomes: T-05 (feature crates)
- Done. The main session ran all compiling crates together on a fresh DB: 603 passed, 0 failed; clippy and fmt clean.
- `InboxService`: `counts(household, viewer)`; `approve_bill/reject_bill/approve_contact/reject_contact` take `viewer: Viewer`; any non-adult viewer (devices too) gets `MailError::Forbidden`. New public guards **`InboxService::{ensure_message, ensure_event, ensure_task}`**: event approve/edit/reject and suggested-task accept/dismiss live in fa-calendar/fa-tasks, so the **API must call these first**.
- `Http::with_policy(allow_http, allow_private_hosts)`, `ConnectorError::PrivateHost` (→ `CalendarError::Invalid`). `ConnectorConfig`, `WeatherConfig`, `MailConfig` have `allow_private_hosts` (default false). `MailDeps::http` is built by the server. IMAP (raw `TcpStream`) is **not** guarded; C-05 covers HTTP only (known gap).
- Advisory clearing after each successful evaluation; nothing is cleared if any active rule fails to parse.
- `approval_needed` goes only to the source's adults (two mail tests now expect one row).
- **Your breakage:** `crates/fa-api/src/mail.rs` ~1060 `inbox.counts(hid)` needs a `Viewer`; inbox approve/reject handlers pass a `Viewer` and are adult-only; call `ensure_event`/`ensure_task` before event and suggested-task actions; test fixtures in `crates/fa-api/tests/*.rs` need `allow_private_hosts: true` in `ConnectorConfig`/`MailConfig`/`WeatherConfig` literals and `Http::with_policy(.., true)` where they hit local stubs.
