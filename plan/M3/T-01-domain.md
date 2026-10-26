---
id: T-01
title: "fa-domain: M3 IDs, enums, entities, DTOs, helpers, authz"
milestone: M3
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-domain/**"]
---

# T-01: fa-domain: M3 IDs, enums, entities, DTOs, helpers, authz

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Domain types*, *Migration* (the column sets your entities mirror), *HTTP API v1 additions* (which DTO each route uses), *Pipeline engine* (`Candidate`, `Subject`)
- `arch/Ingest.md` §3, §6.1, §12; `arch/Domain.md` (Organizations & Mail, RBAC)
- `crates/fa-domain/src`: `entities.rs` (`db_enum!`, `text_enum!`, `SourceRef`), `ids.rs`, `tasks_api.rs` and `feeds.rs` (DTO patterns, ts-rs and utoipa attributes), `authz.rs` (matrix and exhaustive tests), `api.rs` (`HouseholdView`, `MemberView`, patches), `dashboard.rs`

## You own
- `crates/fa-domain/**`

## Work
1. Add the IDs, enums, entities and DTOs listed in *Domain types*, in new files `mail.rs`, `mail_api.rs`, `pipeline.rs` (re-exported from `lib.rs` like the others). Every enum gets `as_str()`/`FromStr`/`ALL` matching the migration's CHECK values (`db_enum!`). `DecisionPointKey::as_str` returns the dotted key (`email.category`).
2. Extend the existing types: `EventStatus` (+`Extracted`, `Expired`), `TaskListKind::Email`, `TaskOrigin::Email`, `NotifyTrigger::{ApprovalNeeded, MailTimeSensitive}`, `Member` (+`aliases`, `gender`, `relevance_tags`), `Household` (+ the M3 settings structs), `NotifyPref` (+`approval_mode`), `HouseholdView`/`HouseholdPatch`, `MemberView` and member patches, `NotifyPrefView`/`NotifyPrefPatch`, `DashboardView` (+`inbox: InboxCounts`). New fields on existing structs will break struct literals in other crates (fa-store, fa-tasks, fa-api, fa-config, tests): **add `#[serde(default)]` where sensible and fix every literal in the other crates with defaults so the workspace builds** (that is the one exception to path ownership in this task; keep those edits minimal and list them in your report). fa-store row mappings may use interim defaults (the fa-store task replaces them).
3. Pure helpers in `fa_domain::mail` (see the plan) with unit tests: deep link, `message_external_url`, address normalising, `domain_matches` (subdomains yes, suffix tricks like `evilexample.com` no), `bill_dedupe_key`, `default_categories()` (exactly the D-054 list), `valid_category_key`.
4. Authz: replace `Resource::MailSource { owner }` with `{ owners, visibility }`; add the actions `ReadMail`, `ManageMailSource`, `ManageMailSetup`, `ReadInbox`, `ManageBills`, keeping `GmailAction` and `ApproveEvent` with the semantics in the plan. Children and devices never `ReadMail`. `ReadInbox`: adult yes; plain device yes (counts only, enforced in services); elevated device yes. `ApproveEvent`: adult, elevated device. Update the matrix tests (they are exhaustive over `Action::ALL`).
5. DTOs derive `Serialize, Deserialize, TS, utoipa::ToSchema` with ts export to `../../../apps/web/src/gen/types/`. Optional fields: `#[ts(optional = nullable)]`, or `#[ts(as = "Option<X>", optional = nullable)]` for named types (never `#[ts(type = "X | null")]`). Each DTO field that is a time or date says its wire format in a doc comment.

## Acceptance
- [ ] `cargo test -p fa-domain` passes
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-domain --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] `cargo test -p fa-domain` regenerates the ts files without errors (don't commit them; the main session regenerates `apps/web/src/gen`)
