---
id: T-07
title: "fa-server: settings loop with restart, wiring, ops"
milestone: M3.5
wave: 4
depends_on: [T-03, T-05, T-06]
agent: {model: sonnet, effort: medium}
status: todo
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
