---
id: T-10
title: "fa-mail processing side: process, extraction, inbox, labelling, setup"
milestone: M3
wave: 4
depends_on: [T-04, T-05]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-mail/src/process/**", "crates/fa-mail/src/process.rs", "crates/fa-mail/src/inbox.rs", "crates/fa-mail/src/labelling.rs", "crates/fa-mail/src/setup.rs", "crates/fa-mail/src/context.rs", "crates/fa-mail/tests/process_*.rs", "crates/fa-store/src/repo/ext_m3_t10.rs"]
---

# T-10: fa-mail processing side: process, extraction, inbox, labelling, setup

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Mail service*, *Pipeline engine*, *Rules*, *Jobs*, hand-off table under *Crates*, *Domain types* (message, inbox, org, rule, category DTOs)
- `arch/Ingest.md` §5–§10, §12; D-008, D-009, D-027, D-028, D-054 to D-063
- `crates/fa-mail/src/lib.rs` (W0), `crates/fa-rules` (T-04), `crates/fa-pipeline` (T-05), `crates/fa-store` (T-03), `crates/fa-tasks/src/service.rs` (`Viewer` pattern)
- T-09 works in parallel in `sync*`, `sources.rs`, `actions.rs`, `retention.rs`, `provider.rs`: don't touch those

## You own
- `crates/fa-mail/src/process/**`
- `crates/fa-mail/src/process.rs`
- `crates/fa-mail/src/inbox.rs`
- `crates/fa-mail/src/labelling.rs`
- `crates/fa-mail/src/setup.rs`
- `crates/fa-mail/src/context.rs`
- `crates/fa-mail/tests/process_*.rs`
- `crates/fa-store/src/repo/ext_m3_t10.rs`

## Work
1. `context.rs`: build `fa_rules::Context` for a household + source from the store (orgs with contacts, members with enrollments, gender terms, rules, disabled signals, categories, owners, tz). Cache per job run.
2. `process.rs` / `process/**`: job `mail.process{message, only?}` exactly as in *Mail service* (T-10 paragraph). The `Evaluator` wraps `fa_rules::evaluate` for the message facts built from the stored message. Write message columns and `message_member` (`rule` or `default`); `mail_time_sensitive` rows; extraction only for `extract` categories and never for `junk`/`ignore`; events → `EventRepo::insert_extracted` + `calendar.route{event}`; bills (`BillRepo::insert_if_new`, ≥ 0.8 `open` + `tasks.from_bill`, else `proposed` + `approval_needed` rows, click `/inbox`); PDFs: `fa_rules::pdf_text` in `spawn_blocking` with a 10 s timeout, only when no amount was found in the body, text stored into `attachments`; tasks per `category.email_tasks` via `tasks.from_mail` (assignees from member links; reply tasks to the adult owners); contact hints per `org.contact_mode` (`suggest` → `ContactSuggestionRepo::insert_if_new` + approval rows; `auto` → create the contact and link it to the member's enrollment at that org); enqueue `mail.auto_actions{message}`; set `processed`; publish `mail.updated` and `inbox.updated` when proposals were created. Re-processing a message (e.g. after a label fix) must not duplicate events, bills, tasks or suggestions (keys and `insert_if_new`).
3. `inbox.rs` (`InboxService`): `list(household, viewer)` merges `EventRepo::proposals`, proposed bills, suggested email tasks (as `TaskView`; you may build them with the existing fa-store task rows — don't depend on fa-tasks) and proposed contacts, newest first, with counts; `approve_bill(BillPatch)` → `open` + `tasks.from_bill`; `reject_bill` → `dismissed`; `approve_contact(ContactApproval)` → contact + enrollment link; `reject_contact`. Every approval/rejection records a `human` decision where a decision point exists (bills: `email.is_bill` true/false on the message).
4. Job `inbox.expire` (cron `7 * * * *`): `EventRepo::expire_past`, suggested email tasks older than `inbox_task_expiry_days` → `skipped`, contact suggestions older than `inbox_contact_expiry_days` → `expired`. Publish `inbox.updated` when anything changed.
5. `labelling.rs` (`LabelService`): `next`, `label` (human decision via `Pipeline::record_human`; update message columns / `message_member` with `user`; set `is_learning_example`; a changed category re-enqueues `mail.process` with `only` = the extraction steps for newly `extract` categories), `set_eval_sample`, progress (target 200).
6. `setup.rs` (`SetupService`): orgs, contacts, enrollments, rules (`validate_rule`; `test_rule` runs a rule over the last N readable messages with content), signals list/toggle (`household.disabled_signals`), categories CRUD (archiving allowed; `key` immutable), `make_task`, `list_messages` (readable sources only; ignored categories excluded unless `include_ignored`), `message_detail` (sections with relevance for `member`, decisions, derived items, actions, `can_act`). `Viewer{member, role, device, elevated}` like fa-tasks.
7. `process::handlers`/`default_schedules`.
8. **Tests** (`#[sqlx::test]`): process the T-04-style fixtures end to end (insert a message row directly): a teacher's ICS mail → category `school`, member linked, one `extracted` event + `calendar.route` job; a newsletter → sections with relevance per child (tags and gender); a bill → `open` + `tasks.from_bill` job, and a low-confidence one → `proposed` + approval rows; a payment-received mail → no bill; a permission slip → `tasks.from_mail` suggest job; an unknown sender at a known org → contact suggestion, and `auto` mode → contact created; promotions → no extraction; re-processing → no duplicates; visibility filter; label → human decision; inbox list/approve/reject; expiry.

## Acceptance
- [ ] `cargo test -p fa-mail` passes
- [ ] `SQLX_OFFLINE=false cargo build --workspace` passes against the dev DB (the main session re-prepares `.sqlx`)
- [ ] `cargo clippy -p fa-mail --all-targets -- -D warnings` and `cargo fmt --check` are clean (T-09 works in the same crate: if its half doesn't compile yet, report rather than editing it)

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
