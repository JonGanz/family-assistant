---
id: T-01
title: "fa-domain: M1 types, view(), policy, ToSchema DTOs"
milestone: M1
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-domain/**"]
---

# T-01: fa-domain: M1 types, view(), policy, ToSchema DTOs

You are a sub-agent implementing one task of the M1 milestone of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- plan/M1.md: *Domain types*, *HTTP API v1 additions*, *Process config*
- arch/Feeds.md §2 (data model), §3.5 (visibility), §4.1 (rule enums only; the DSL itself is T-06)
- arch/Frontend.md §4.1 (the `DashboardSettings` shape)
- crates/fa-domain/src (ids.rs, entities.rs, authz.rs, validation.rs)

## You own
- `crates/fa-domain/**`

## Work
1. **IDs** (ids.rs, following the existing newtype pattern): `OAuthAccountId`, `CalendarSourceId`, `CalendarId`, `EventId`, `WeatherRuleId`, `AdvisoryId`.
2. **Enums** (`sqlx::Type` text mapping matching the CHECK values in Feeds.md §2, plus serde snake_case):
   - `Units {Metric, Imperial}`
   - `SourceKind {Google, Caldav, Ics}`
   - `SourceState {Ok, NeedsReauth, Error}`
   - `Visibility {Full, Busy}` (D-036)
   - `EventOrigin {Synced, Extracted, Manual}`
   - `EventStatus {Proposed, Approved, Written, Rejected}`
   - `RuleAction {Alert, Task}`
   - `Severity {Info, Warning, Severe}`
   - `RuleStatus {Active, Suggested, Disabled}`
   - `SuggestedBy {Default, S2, User}`
   - `AdvisorySource {Rule, S2}`
3. **Entities** for the M1 tables in Feeds.md §2 (`OAuthAccount`, `CalendarSource`, `Calendar`, `Event`, `WeatherSnapshot`, `WeatherRule`, `Advisory`, `MemberAvatar` metadata), and extend `Household` with `units`, `calendar_past_days`, `calendar_future_days`, `weather_retention_days`, `private_events_as_busy`. Secrets are never fields: use `secret_id: Option<SecretId>` style references only.
4. **`DashboardSettings`** (Frontend.md §4.1):
   - Fields: `orientation`, `theme`, `agenda_days`, `members` (`All` or a list of IDs; serialised as the string `"all"` or a JSON array of IDs, via serde untagged), and `panels: Vec<Panel{kind: String, …extra}>`.
   - Unknown panel kinds are kept as they are, so round trips don't lose them.
   - `validate()`: agenda_days 1..=14, at most 12 panels, known orientation and theme.
   - `Default` is the "Family hub" preset.
5. **Visibility:** the module `calendar` with the pure `view(event: &EventMeta, cal: &CalendarMeta, viewer: &Viewer, flags: &HouseholdFlags) -> Visibility`, implementing the Feeds.md §3.5 table (D-036: adult-owned private events are Busy for children and the dashboard when the flag is on; adults always see Full):
   - `EventMeta{is_private}`
   - `CalendarMeta{members: &[(MemberId, Role)], owner_role: Option<Role>}`
   - `Viewer = AdultUser | ChildUser(MemberId) | Device | ElevatedDevice`
   - `HouseholdFlags{private_events_as_busy}`

   Also add a `mask(EventView) -> EventView` helper that blanks title, location, and description to "Busy" for `Busy`.
5b. **Trait:** `fa_domain::AccessTokenProvider` (`#[async_trait] fn access_token(&self, household, OAuthAccountId) -> Result<String, TokenError{NeedsReauth, Other(String)}>`) in a new `oauth` module (async-trait is a workspace dependency). fa-auth implements it and fa-calendar consumes it.
6. **Policy** (authz.rs): add the actions `ManageCalendarSource` (resource `Source{owner: Option<MemberId>}`), `ReadCalendar`, `ManageWeatherRules`, `DismissAdvisory`, and `ManageDeviceSettings`, and update `Action::ALL`.
   - **ManageCalendarSource:** an adult may manage household sources (owner `None`), their own, and children's, but never another adult's (D-006). Resolve the owner's role through the resource, as `Resource::Member(id, role)` does.
   - **ReadCalendar:** any user or device.
   - **ManageWeatherRules** and **ManageDeviceSettings:** adult users only, not devices, not even elevated (D-019: no settings on the dashboard).
   - **DismissAdvisory:** any user or device.
   - Extend the exhaustive policy tests.
7. **DTOs:** every request and response type in the M1 API table, named exactly as the table names them (`HouseholdView`, `HouseholdPatch`, `OAuthAccountView`, `SourceView`, `NewSource` as a tagged enum `{kind: "google"|"caldav"|"ics", …}`, `SourcePatch`, `CalendarView`, `CalendarPatch`, `EventView`, `WeatherView`, `AdvisoryView`, `WeatherRuleView` + create and patch inputs, `ValidateRuleRequest/Response`, `DeviceView` with `settings`, `DashboardView`, `StatusView`). `MemberPublic` and `MemberFull` gain `avatar_url: Option<String>` (see plan/M1.md). All derive `Serialize/Deserialize` and `utoipa::ToSchema`.
   - **M0 DTOs** also gain `ToSchema`.
   - `MemberView` becomes an untagged union of `MemberPublic` and `MemberFull`.
   - `SourceView.masked_url` is computed by a `mask_url(&str) -> String` helper (`https://host/…`). Test it.
8. **Tests:** a `view()` truth table over every viewer × visibility × private × membership combination, policy additions, `DashboardSettings` validation and unknown-panel round trip, `mask_url`, and serde round trips for `NewSource` variants.

## Acceptance
- [ ] `cargo test -p fa-domain` passes
- [ ] `cargo clippy -p fa-domain --all-targets -- -D warnings` passes
- [ ] `cargo build --workspace` still builds (M0 crates compile against the extended types; report any break you cannot fix inside your crate)
- [ ] `cargo fmt --check` is clean

## Ground rules (all M1 tasks)
- Repo: `/home/jon/projects/family-assistant`. **First read** `CLAUDE.md`, `plan/M1.md` (especially *Contracts fixed up front*), and the arch sections listed under *Read first*. The contracts in `plan/M1.md` are the source of truth. If one is wrong or missing, don't invent a new one: work around it minimally and report it.
- **Edit only the paths you own.** The root `Cargo.toml` belongs to the main session. Add dependencies to your own crate only as `workspace = true` entries (pre-declared in W0). If one is missing, report it.
- **Disk:** run `df -h /` before building. If less than 6 GB is free, stop and report.
- External names: verify crate features with `cargo add --dry-run <crate> --features …` before you rely on them. Never guess API paths. Use arch/Feeds.md or the vendor docs.
- Code style: idiomatic Rust 2024, with `thiserror` in libraries, `anyhow` only in bins, no `unwrap()` outside tests, and `tracing` for logs (never log secrets, tokens, or full feed URLs). Keep comments sparse. Match the surrounding code (for example the `XxxRepo` associated functions taking `&mut PgConnection` and returning `Versioned<T>`).
- Always run `cargo fmt` (and `pnpm format` for web) before reporting.
- No hand-written API types in the web app: `apps/web/src/gen` is generated by the main session.
- Public AGPL repo (D-035): no real personal data. Fixtures use `example.com` and fictional names.
- Tools: pnpm is `corepack pnpm@9`. `just` and `psql` aren't installed (use `docker compose -f deploy/compose.dev.yaml exec postgres psql -U fa`). Dev DB: `postgres://fa:fa@127.0.0.1:55432/fa`.
- Don't commit, and don't run `git` commands that change state.
- **Report** (your final message), kept concise:
  1. what you built (files);
  2. the acceptance command results (pass or fail, with output on failure; quote counts only from actual output);
  3. any deviations from the arch docs or contracts, and why;
  4. follow-ups or requests for other tasks' paths.
