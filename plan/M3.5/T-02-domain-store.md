---
id: T-02
title: "fa-domain + fa-store: migration, types, DTOs, authz, repos"
milestone: M3.5
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-domain/**", "crates/fa-store/**"]
decisions: [D-065, D-068, D-070, D-073, D-074, D-075, D-078]
---

# T-02: fa-domain + fa-store: migration, types, DTOs, authz, repos

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `plan/M3.5.md`: *Contracts fixed up front* → Database, fa-domain, fa-store (exact names)
- `arch/Platform.md` §6, `arch/Feeds.md` (advisory lifecycle), `arch/Ingest.md` §10 and §12, `arch/Domain.md` (RBAC)
- `crates/fa-store/src/secret.rs` (envelope encryption to reuse), `crates/fa-store/src/repo/{household,advisory,notification,bill,mail_source,member,account}.rs`, `crates/fa-domain/src/{api,authz,entities}.rs`
- IMPROVEMENTS.md C-01b, C-02 (file and line references)

## You own
- `crates/fa-domain/**`
- `crates/fa-store/**`

## Work
1. **Migration** `20261028000001_m35_onboarding.{up,down}.sql` exactly as in the contract: `instance_setting` (no household RLS; `GRANT SELECT, INSERT, UPDATE, DELETE` to `fa_app`), `household.dashboard_pending_approvals`, `household.setup_completed_at` (set to `now()` for existing rows), drop `advisory.dismissed_at/_by`. Never edit earlier migrations.
2. **fa-domain:** the `Household`, `HouseholdPatch`, `Advisory`, `DashboardView` changes; the new `system_api` module with every type in the contract table (derive like the neighbouring DTOs, including `ToSchema` and the ts export); `authz` changes (`DismissAdvisory` removed; every inbox action refused for device actors, elevated or not; `ManageMember` adult-only). Update the exhaustive authz tests.
3. **fa-store:** `InstanceSettingRepo` (secrets via the `secret.rs` primitives, never logged), `SetupRepo`, `AdvisoryRepo::retain_rule_advisories` (and `dismiss` removed), `NotificationRepo::insert_for_source_adults`, `InboxRepo::pending_by_adult` (events and bills link through `message_id`, contact suggestions through `message_id`, email tasks through `task.source_ref->>'message_id'`; follow `message.source_id` to the source's owners and visibility), household row mapping and `update_settings` for the two new columns. `notify_changed` uses `pg_notify('fa_settings', '')`.
4. **C-01b, C-02:** lockout reads `FOR UPDATE` and conditional updates, with a concurrency test (two parallel failed attempts both counted).
5. Remove the M3 "interim" `inbox` mapping in the dashboard data path if it is in your paths; everything else that breaks outside your paths is listed in your report (fa-api, fa-config, fa-server, fa-weather, fa-mail are fixed in later waves).
6. `cargo sqlx prepare --workspace` (with `SQLX_OFFLINE=false`) if any `query!` macro changed; it may fail on crates outside yours that no longer compile: then prepare with `-p fa-store` and report.

## Acceptance
- [ ] `cargo test -p fa-domain` and `cargo test -p fa-store` pass on the dev DB (schema only) **and** on a fresh container (port 55402)
- [ ] tests: secret settings round-trip and are unreadable without the key; `pending_by_adult` counts an owners-only source for its owner only and a household source for every adult; `insert_for_source_adults(None)` reaches every adult; `retain_rule_advisories` keeps the listed keys and `s2` advisories
- [ ] `cargo clippy -p fa-domain -p fa-store --all-targets -- -D warnings` and `cargo fmt --check` clean
