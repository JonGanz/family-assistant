---
id: T-11
title: "fa-calendar: route, gate, approve, write, cancel"
milestone: M3
wave: 4
depends_on: [T-02, T-05]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-calendar/**", "crates/fa-store/src/repo/ext_m3_t11.rs"]
---

# T-11: fa-calendar: route, gate, approve, write, cancel

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *fa-calendar additions*, *Pipeline engine*, *Jobs*, *Domain types* (`EventProposalView`, `EventApproval`)
- `arch/Ingest.md` §8, §10; D-008, D-057, D-061
- `crates/fa-calendar/src` (service, jobs, remote: token and CalDAV client patterns), `crates/fa-connectors` (T-02's `insert_event`, `put_event`, `delete_event`), `crates/fa-pipeline` (T-05), fa-calendar must **not** depend on `fa-rules`: the dedupe key is already stored on the row by fa-mail; implement the title similarity with `strsim` (add `strsim.workspace = true`) using the same normalisation as `fa_rules::title_similarity`

## You own
- `crates/fa-calendar/**`
- `crates/fa-store/src/repo/ext_m3_t11.rs`

## Work
1. Add `fa-pipeline` (and `strsim`) as dependencies. Job handlers `calendar.route`, `calendar.write`, `calendar.delete` (registered in `handlers()`).
2. `calendar.route` exactly as in the plan: dedupe, cancellations, routing evaluator (only calendars on writable, enabled sources), gate with `household.events_*`, `approval_needed` rows (key prefix `approval:event:{event}`, click `/inbox`), publish `inbox.updated` and `calendar.updated` as appropriate.
3. `CalendarService`: `proposals`, `approve_event` (apply `EventApproval` edits; the calendar must be writable; human decision for `event.calendar_route` with the chosen calendar), `reject_event`, `approve_cancel`, `reject_cancel`.
4. `calendar.write` / `calendar.delete` per the plan (Google: `calendar.events` scope check with the account's `scopes`; CalDAV: our UID `fa-<event id>@family-assistant`). After a write, the row is `written` with `calendar_id`, `remote_uid`, etag; a following `calendar.sync` must not duplicate it (T-03 fixed `upsert_synced`; add a test here that syncs after a write with wiremock).
5. Tests (`#[sqlx::test]` + wiremock): dedupe by key, by similarity, by UID; cancellation of a proposed and of a written event; routing confidences; gate off → proposed (default), gate on → approved + write job; approve with edits → Google insert called with the edited fields; CalDAV PUT; write error kept on the row; reject.

## Acceptance
- [ ] `cargo test -p fa-calendar` passes
- [ ] `SQLX_OFFLINE=false cargo build --workspace` passes
- [ ] `cargo clippy -p fa-calendar --all-targets -- -D warnings` and `cargo fmt --check` are clean (build against the dev DB; the main session re-prepares `.sqlx`)

## Wave 1 outcomes (read before starting)
- T-01 is done (`cargo test -p fa-domain`: 247 passed). New files `fa-domain/src/{mail,mail_api,pipeline}.rs`. `DecisionPointKey` serialises as the dotted key and has `default_value()`. Settings structs `GenderTerms`, `EventsSettings`, `InboxSettings`, `MailRetention` have `Default` matching the migration; `DEFAULT_CURRENCY`, `DEFAULT_BILL_LEAD_DAYS`. `Message.attachments`/`headers` are `serde_json::Value` (typed element: `AttachmentMeta`). `Resource::MailSource{owners, visibility}`; new actions `ReadMail`, `ManageMailSource`, `ManageMailSetup`, `ReadInbox`, `ManageBills`.
- **Interim defaults** (marked with comments) exist in `fa-store/src/repo/household.rs` and `member.rs` row mappings, `fa-notify/src/service.rs` (`approval_mode`), `fa-api/src/feeds.rs` (`DashboardView.inbox`), `fa-config/src/household.rs`. Replace the ones in your paths.
- T-02 is done: `fa_connectors::google::gmail::*`, `fa_connectors::imap::{ImapClient, ImapConfig, …}` (`internal_date: Option<…>`; `uid_store_flags`/`gm_labels(uid, add: bool, &[&str])`; `capabilities()` is async), `google::calendar::{insert_event, patch_event, delete_event}` returning `Written{id, ical_uid, etag, html_link?}`, `CaldavClient::{put_event, delete_event}`, `ics::build_vevent`, `EventWrite{start, end: DateTime<Utc>, all_day, tz}`. Errors `HistoryExpired`, `InsufficientScope`, `Retryable(u16)`, `AuthFailed`, `Imap(String)`. A `Plain` IMAP connection is made whenever the caller asks for it: callers enforce `allow_plain_imap`.

## Wave 2 outcomes: T-04 (fa-rules)
- Done (`cargo test -p fa-rules`: 15 unit + 26 integration passed). Types in `fa_rules::{MessageFacts, Context, OrgCtx, ContactCtx, MemberCtx, EnrollCtx, AttachmentFacts, ParsedMessage, ParsedAttachment, Section, SignalDef, RuleError}`. Header names in `MessageFacts.headers` are lower-case.
- `member_link` candidate values are member-id strings. Member signals apply to children only; grade/tag/gender/"all children" signals need an enrollment at the matched org. Build `Context` with enrollments already filtered to those valid today and contact emails in `contacts`.
- `extract_bills` ignores "payment received": **gate bill extraction on the `email.is_bill` decision**. `extract_bills` resolves due dates in UTC (no tz argument). `pdf_text` refuses > 5 MB and catches parser panics; run it in `spawn_blocking` with a 10 s timeout, only when the body has no amount.
- Date-phrase events are skipped when ICS or schema.org events exist; ≤ 10 per message. `ContactHint.member` is a `MemberId` (not optional).

## Wave 2 outcomes: T-03 (fa-store)
- Done (fa-store tests pass on the dev DB and a fresh container). New repo files under `crates/fa-store/src/repo/`: `mail_defaults.rs` (`mail_defaults::ensure`), `org.rs`, `mail_source.rs`, `message.rs`, `pipeline.rs`, `mail_setup.rs`, `bill.rs` (`BillRepo`, `MailActionRepo`, `InboxRepo::counts`), `event_m3.rs`, `task_m3.rs`. Your `ext_m3_tNN.rs` file exists and is registered. New repos use runtime `sqlx::query_as` + `FromRow` (no `.sqlx` needed for those): prefer that style in your ext file too.
- Signature notes: `NotificationRepo::insert_for_adults` has **no household parameter** (uses the tx household; needs `Store::tx_for`, inserts nothing in a system tx) and returns the row count. `MessageRepo::label_queue(conn, household, sources, exclude, limit)` and `eval_progress(conn, household)`. `MessageFilter.include_ignored_keys` = category keys to **hide** (`None` hides nothing); pass the `ignore`-behaviour keys unless `include_ignored`. `MessageRepo::get` returns metadata only; `get_with_content` everything. `set_classification` takes `Option`s (None = unchanged). `set_sync_state` keeps `history_id`/`last_synced_at` on None; use `MailSourceRepo::clear_history_id` for the Gmail 404 fallback. `DecisionRepo::{latest, latest_final, latest_batch}`. `TaskListRepo::get_or_create(kind, name, &defaults: TaskList)`. `EventRepo::set_written` removes a same-uid synced row first. Extras: `MessageRepo::{get_by_remote, members_of, pending}`, `MailSourceRepo::{message_counts, set_owners, clear_history_id}`, `PipelineRepo::{stages, activate}`, `HouseholdRepo::{disabled_signals, set_disabled_signals}`.
- `NotifyPref.approval_mode` exists; interim `approval_mode: Default::default()` literals sit in `fa-notify/src/service.rs`, `fa-config/src/tasks.rs`, `fa-tasks/tests/tasks.rs` (owners: replace with real values).

## Wave 3 outcomes: T-05 (fa-pipeline)
- Done (6 tests). `fa_pipeline::{Pipeline, Evaluator, Outcome, validate_version, PipelineError}`; `Candidate` re-exported from fa-domain. `Pipeline` is a unit struct: `Pipeline::new().run(conn, household, dp, subject, &eval)`, `record_human` (confidence 1.0, source "human"), `current`/`all_current` (one decision per dp). **For multi-valued `email.member_link` read every value with `DecisionRepo::latest_batch`.**
- Recency is by `created_at` (the transaction timestamp): call `run` and `record_human` for the same subject in **separate transactions**.
- Wave 3 runs T-06 (fa-tasks: `tasks.from_mail`, `tasks.from_bill`), T-07 (fa-notify) and T-08 (fa-config) in parallel with you; rely only on the job payloads and kinds in the plan.
