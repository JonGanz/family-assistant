---
id: T-03
title: "fa-auth + fa-api + fa-config: hues over the wire, TOML, theme in device settings"
milestone: M3.6
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-auth/**", "crates/fa-api/**", "crates/fa-config/**", "config/**"]
decisions: [D-080, D-081]
---

# T-03: fa-auth + fa-api + fa-config: hues over the wire, TOML, theme in device settings

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Backend wire formats*; T-02's report (added below when done)
- `crates/fa-auth/src/own.rs`, `crates/fa-api/src/routes.rs`, `crates/fa-config/src/household.rs`, `config/examples/household.example.toml`

## You own
- `crates/fa-auth/**`, `crates/fa-api/**`, `crates/fa-config/**`
- `config/**` (the example household file is included by tests)

## Work
1. Replace the `#rrggbb` check in `fa-auth` (`own.rs`, setup owner) with the `Hue` type: no string validation is left for member colours.
2. `fa-api`: member create and patch request types and any dashboard or member view use `Hue`; a non-hue value is `422`. Register `Hue`, `ThemeChoice` and `ThemeMode` in OpenAPI; make sure the device-settings routes accept and return the new `theme` object (and the legacy string on input).
3. `fa-config`: `MemberConfig.color: Option<Hue>`. Import also accepts a legacy `#rrggbb` string for a **member** and maps it to `blue`; export writes the name. Calendar and category colours in TOML stay hex. Update `config/examples/household.example.toml` (member colours become hue names, one of each is fine) and any import/export round-trip tests.
4. Fix the tests in your crates that use hex member colours (grep `color` in `crates/fa-auth/tests`, `crates/fa-api/tests`, `crates/fa-config/tests`).

## Acceptance
- [ ] `cargo test -p fa-auth -p fa-api -p fa-config` pass (fresh container port 55403)
- [ ] `cargo clippy -p fa-auth -p fa-api -p fa-config --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] new tests: creating a member with `"purple"` works and `"#ff0000"` gives 422; patching to `null` clears it; a legacy household TOML with a hex member colour imports as `blue`; export then import round-trips the names; a device saved with `"theme": "dark"` returns the object form; the setup owner takes a hue
- [ ] report: anything else outside your paths that still uses a string member colour

## Outcome (T-03 report)
- Member colours are `Hue` in the API (422 for anything else), the setup owner and TOML (`hue_or_legacy_hex`: a legacy `#...` string imports as `blue`, any other non-hue string is an error; export writes the name). `Hue`, `ThemeChoice` and `ThemeMode` are in OpenAPI. The device-settings routes needed no change. Example household file updated.
- fa-config and fa-api build locally only with `--ignore-rust-version` (local rustc 1.96, `mail-parser` wants 1.98); `fa-server gen-openapi` ran the same way.
- Main session: regenerated `openapi.json` and `api.d.ts`, removed the `as never` in `LayoutEditor.vue`, fixed the stale theme fixture in `dashboard/tasks.test.ts`.
