---
id: T-02
title: "fa-domain + fa-store: Hue, ThemeChoice, member colour migration"
milestone: M3.6
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-domain/**", "crates/fa-store/**"]
decisions: [D-080, D-081]
---

# T-02: fa-domain + fa-store: Hue, ThemeChoice, member colour migration

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.6/_rules.md`
- `plan/M3.6.md`: *Backend wire formats*
- `decisions/D-080-member-hue-colours.md`, `decisions/D-081-theme-per-device.md`
- `crates/fa-domain/src/{entities,dashboard,api,system_api}.rs`, `crates/fa-store/src/` (the member repo) and the latest migration

## You own
- `crates/fa-domain/**`
- `crates/fa-store/**`

## Work
1. New module `fa_domain::theme` with `Hue`, `ThemeChoice`, `ThemeMode` and `DEFAULT_THEME_FAMILY`, exactly as in the plan (derives, serde, `ToSchema`, ts-rs export to `apps/web/src/gen/types/`).
2. Change every **member** colour to `Option<Hue>`: `Member`, `MemberDto`, `MemberPublicDto`, the member create and patch inputs in `fa-domain`, and `NewSetupOwner`. Do not touch calendar, task-list or category colours.
3. `DashboardSettings.theme` becomes `ThemeChoice`; remove `DashboardTheme`. Implement the legacy-string-or-object deserialisation and the id validation (`SettingsError::ThemeId`).
4. The migration from the plan, and the member repo reading and writing the hue (store the lowercase name as text; reading an unknown name is an error, not a panic).
5. If you touch `query!` macros, regenerate `.sqlx` with `cargo sqlx prepare --workspace` and `SQLX_OFFLINE=false` (see the M3.5 rules: if it fails or deletes files, stop and report). Prefer runtime `query_as` for anything new.
6. Fix the tests in your crates that build members with a hex colour.

## Acceptance
- [ ] `cargo test -p fa-domain -p fa-store` pass (fresh container port 55402)
- [ ] `cargo clippy -p fa-domain -p fa-store --all-targets -- -D warnings`, `cargo fmt --check`
- [ ] new tests: `Hue` round-trips and rejects `"#ff0000"` and `"teal"`; `ThemeChoice` reads the legacy `"dark"` string and the object, rejects `"neon"` and an id with an upper-case letter or a space, and always writes the object; the migration turns existing hex members into `blue` and the constraint rejects `"#ff0000"`; `Hue::DEFAULT` is blue
- [ ] report: list the crates outside your paths that no longer compile because of the type change (T-03 and the main session fix them)

## Outcome (T-02 report)
- Built `fa_domain::theme` (`Hue`, `ThemeMode`, `ThemeChoice` with a hand-written `Deserialize` for the legacy string, `DEFAULT_THEME_FAMILY`, `is_theme_id`), `DashboardSettings.theme: ThemeChoice` with `SettingsError::ThemeId`, member colours as `Option<Hue>` (`Member`, `MemberDto`, `MemberPublicDto`, `NewSetupOwner`), the migration `20261029000001_m36_member_hue`, the member repo, and `crates/fa-store/tests/m36.rs`.
- No member create/patch inputs exist in fa-domain; they are in fa-api (T-03).
- `.sqlx` unchanged. Running the tests regenerated `apps/web/src/gen/types/{Hue,ThemeChoice,ThemeMode}.ts` and updated three member types.
- Not yet compiled by T-02: fa-api, fa-config, fa-auth, fa-server uses of member colour and `DashboardTheme`.
