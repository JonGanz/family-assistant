---
id: T-03
title: "fa-config: layered runtime settings, cookie mode, DB password file, create_household"
milestone: M3.5
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-config/**", "config/**"]
decisions: [D-065, D-069, D-072, D-075]
---

# T-03: fa-config: layered runtime settings, cookie mode, DB password file, create_household

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: *Contracts fixed up front* → fa-config, and the precedence rule in `plan/M3.5/_rules.md`
- `arch/Platform.md` §6
- `crates/fa-config/src/{process,household,lib}.rs`; `bins/fa-server/src/serve.rs` (`trusted_proxies`, `lan_config`, `lan_base_url`, `env_flag`: the raw env reads you absorb)
- T-02's report (in this brief's *Wave outcomes* section, added by the main session)

## You own
- `crates/fa-config/**`
- `config/**`

## Work
1. `ProcessConfig` changes from the contract. Keep every existing `FA_*` name working. Remove the https-unless-localhost check; keep the Google client ID/secret pair check.
2. `FA_DATABASE_PASSWORD_FILE`: read the file (trim a trailing newline), percent-encode it, insert it into `database_url` when that URL has no password; error clearly if the file is unreadable.
3. `runtime` module: `RUNTIME_KEYS`, `load_layered`, `validate_value` (URLs must be http or https with a host; CIDR lists parse; bools; text trimmed; secrets non-empty strings), `RuntimeSnapshot` with per-key `SettingSource` and `views()`. `FA_DEFAULT_<KEY>` sets a default without locking. Test each precedence level and that env-set keys report `locked`.
4. `create_household(store, &NewSetupHousehold) -> HouseholdId`: extract the creation side effects of `import` (read it carefully: defaults, seeds) into this function and have `import` call it.
5. `import(..., dry_run: bool)`: a dry run validates and executes inside the transaction, builds the report, then rolls back. `ImportReport` derives `ToSchema` (utoipa is a workspace dependency).
6. Keep `config/examples/household.example.toml` valid; add nothing household-specific.

## Acceptance
- [ ] `cargo test -p fa-config` passes (fresh container port 55403 for DB tests)
- [ ] tests: precedence (env > db > `FA_DEFAULT_*` > built-in), locked keys, password-file insertion with special characters, dry run leaves no rows, import still round-trips
- [ ] `cargo clippy -p fa-config --all-targets -- -D warnings` and `cargo fmt --check` clean
