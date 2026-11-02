---
id: T-02
title: "fa-store: bind secrets to their owner with AES-GCM AAD (C-10)"
milestone: M3.7
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-store/**", "crates/fa-jobs/tests/**", "bins/fa-server/**", "crates/fa-api/src/system.rs", "crates/fa-api/tests/**", "crates/fa-calendar/tests/**", "crates/fa-mail/tests/**", "crates/fa-auth/tests/**"]
decisions: [D-035]
---

# T-02: fa-store: bind secrets to their owner with AES-GCM AAD (C-10)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md`
- `plan/M3.7.md`: *C-10 secrets AAD*
- `crates/fa-store/src/secret.rs`, `crates/fa-store/src/repo/secrets.rs` (note `rewrap_all`), `crates/fa-store/src/repo/instance_setting.rs`, `crates/fa-store/src/lib.rs` (the `seal`/`open` call sites), `crates/fa-api/src/system.rs` (where the server loads the master key), the latest migration in `crates/fa-store/migrations/`
- The finding: AES-GCM secrets carry no AAD, so someone with DB write access can move a ciphertext row between owners (for example attach one member's OAuth token to another member) and the app decrypts it happily.

## You own
The paths in `owns` above. Other crates that call `seal`/`open` and stop compiling: fix the call (add the AAD) and list the file in your report. Do not touch `crates/fa-api/src/*.rs` other than `system.rs`.

## Work
1. Change `MasterKey::seal` and `open` to take `aad: &[u8]` as in the plan. Keep `rewrap` as is (DEK wrap AAD = key version).
2. Helpers in `fa-store` that build the AAD strings for a secret row and an instance setting, exactly as in the plan. Use them from `SecretRepo` and `InstanceSettingRepo` (put, get and the rewrap pass).
3. The migration `20261102000001_secret_aad.{up,down}.sql` adding `aad_version` to both tables (down drops the columns).
4. `aad_version` read and written by both repos; `get` opens with empty AAD when the row's version is 0. Convert the touched `query!` macros to runtime queries (rule in `_rules.md`).
5. `SecretRepo::rebind_all` and `InstanceSettingRepo::rebind_all` as in the plan; `fa-server` runs both after migrations, logging the counts, non-fatal on error. Find where migrations run in `bins/fa-server` and put it right after; if the server only runs migrations in `fa-api` or `fa-store`, put it there and say so.
6. Fix every test that calls `seal`/`open`.

## Acceptance
- [ ] `cargo test -p fa-store` passes against a fresh container (port 55402), including new tests: a sealed secret opens with its AAD and fails with a different one; a row whose `owner_id` is updated by SQL no longer opens; `rebind_all` turns a version-0 row (inserted by SQL with a seal made with empty AAD) into a version-1 row that opens; after `rebind_all` a second call reports 0.
- [ ] `cargo clippy -p fa-store -p fa-server --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes (no new `.sqlx` files needed)
- [ ] `cargo test -p fa-api -p fa-jobs -p fa-calendar -p fa-mail -p fa-auth` pass (they build fixtures with the master key), ports 55412 onwards if you need several containers; report what you ran
- [ ] report: file list, the startup location of the rebind pass, anything in `crates/fa-api/src` (other than `system.rs`) that needed a change
