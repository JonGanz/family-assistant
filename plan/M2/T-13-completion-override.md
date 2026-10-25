---
id: T-13
title: "Follow-up: household dashboard completion override (API, TOML, UI, default on)"
milestone: M2
wave: 7
depends_on: [T-12]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-domain/src/api.rs", "crates/fa-store/migrations/20261026000002_*", "crates/fa-store/src/repo/household.rs", "crates/fa-store/tests/**", "crates/fa-api/src/feeds.rs", "crates/fa-api/tests/**", "crates/fa-config/**", "apps/web/src/views/SettingsHouseholdView.vue", "apps/web/src/views/views.test.ts", "apps/web/src/stores/household.ts", "e2e/**", "README.md", "examples/**", ".sqlx/**"]
---

# T-13: household dashboard completion override

You are a sub-agent finishing one gap in the M2 milestone (Tasks) of the Family Assistant project (Rust backend, Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules: `cargo fmt`, `CARGO_INCREMENTAL=0`, disk checks, `docker rm -f -v`)
- `arch/Tasks.md` §4.3 (the dashboard completion override is **on by default**)
- `crates/fa-store/migrations/20261026000001_m2_tasks.up.sql` (column `household.dashboard_completion_override`, default `false` — wrong)

## Problem
`household.dashboard_completion_override` has no API, TOML key, or UI toggle, and its default contradicts the arch. Without it, dashboard panels are read-only and device completion returns 403.

## Work
1. **Migration:** do NOT edit the M2 migration. Add `20261026000002_m2_override_default.{up,down}.sql`: `ALTER TABLE household ALTER COLUMN dashboard_completion_override SET DEFAULT true` (down: back to false). Do not update existing rows.
2. **Domain/API:** add `dashboard_completion_override` to `HouseholdView` (check whether it's already there) and `HouseholdPatch` in `crates/fa-domain/src/api.rs` (keep ts-rs/utoipa derives consistent with neighbours); `PATCH /household` (adult only, as today) persists it in `crates/fa-store/src/repo/household.rs` and `crates/fa-api/src/feeds.rs`. Add an fa-api test: adult toggles off → device complete 403; on → 200.
3. **TOML:** `[household] dashboard_completion_override = true|false` (optional; omitted = leave as is / default on for new households) in `crates/fa-config`, with a test.
4. **UI:** a toggle in `SettingsHouseholdView.vue` ("Allow ticking off tasks on dashboards"), saved through the household store; a test in `views.test.ts`.
5. **e2e/README:** in `e2e/` replace the `psql` flip of the override with the API `PATCH /household` (keep a spec that turns it off and checks 403, then on). In `README.md`, replace the paragraph saying it is off / has no settings screen (and the override SQL step) with the Settings → Household toggle and the TOML key. Add the key to the TOML example(s) in `examples/` if a household TOML lives there.
6. Regenerate `.sqlx` if queries changed: `cargo sqlx prepare --workspace` against a throwaway Postgres (see `_rules.md`), not by reading the dev DB's rows. Do NOT regenerate `apps/web/src/gen` (the main session does) — but do run `cargo test -p fa-domain` so ts-rs bindings export, then the main session runs gen.

## Acceptance
- [ ] `cargo test -p fa-store -p fa-api -p fa-config -p fa-domain` pass against a fresh Postgres with `SQLX_OFFLINE=true`
- [ ] `cargo clippy --workspace --all-targets -- -D warnings`, `cargo fmt --check` clean
- [ ] `cd apps/web && pnpm test && pnpm run lint` (typecheck may fail until gen is regenerated; say so if it does)
- [ ] Report: files changed, README changes made
