---
id: T-05
title: "fa-pipeline: engine, Rules-only preset, decisions"
milestone: M3
wave: 3
depends_on: [T-03]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-pipeline/**", "crates/fa-store/src/repo/ext_m3_t05.rs"]
---

# T-05: fa-pipeline: engine, Rules-only preset, decisions

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Pipeline engine*, *Seeding*, *Migration* (`decision_point`, `pipeline_version`, `pipeline_stage`, `decision`)
- `arch/Ingest.md` §5; `arch/Domain.md` (AI & Pipelines); D-027, D-029 in `decisions/`
- `crates/fa-store/src/repo` (T-03's `DecisionPointRepo`, `PipelineRepo`, `DecisionRepo`, `mail_defaults`)

## You own
- `crates/fa-pipeline/**`
- `crates/fa-store/src/repo/ext_m3_t05.rs`

## Work
1. Implement the API in *Pipeline engine* exactly (`Pipeline`, `Evaluator`, `Outcome`, `PipelineError` already exists in `lib.rs`; extend it). Functions take `&mut PgConnection` so callers control the transaction.
2. Stage semantics: `rules` → evaluator candidates, threshold per stage; multi-valued `email.member_link` (every candidate ≥ threshold, deduped by member, highest confidence kept); other dps take the single best. `human` → default value from `decision_point.default_value`, `is_final = false`. Unavailable kinds (`knn`, `system_one`, `llm`) per `on_unavailable` (all behave as skip in M3; log once per run at debug). Disabled stages are skipped.
3. `record_human` inserts an `is_final = true` human decision (it supersedes earlier ones by recency). `current`/`all_current` return the latest final decision per dp, else the latest.
4. `validate_version(stages)`: at least one enabled stage, last enabled stage is `rules` or `human`, thresholds in 0..=1.
5. Extra repo methods, if needed, go in `ext_m3_t05.rs`.
6. Tests (`#[sqlx::test]` with the migrations): accepted rules result; fall-through to human default; member_link multi-value; disabled stage skipped; unavailable stage skipped; `record_human` then `current`; `validate_version` cases.

## Acceptance
- [ ] `cargo test -p fa-pipeline` passes
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-pipeline --all-targets -- -D warnings` and `cargo fmt --check` are clean

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
