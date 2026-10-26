---
id: T-09
title: "fa-mail sync side: sources, providers, sync, actions, retention"
milestone: M3
wave: 4
depends_on: [T-02, T-05]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-mail/src/sync/**", "crates/fa-mail/src/sync.rs", "crates/fa-mail/src/sources.rs", "crates/fa-mail/src/actions.rs", "crates/fa-mail/src/retention.rs", "crates/fa-mail/src/provider.rs", "crates/fa-mail/tests/sync_*.rs", "crates/fa-store/src/repo/ext_m3_t09.rs"]
---

# T-09: fa-mail sync side: sources, providers, sync, actions, retention

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Mail service* (both halves, so you know the boundary), *Jobs*, *Process config*, *Domain types* (source DTOs)
- `arch/Ingest.md` §4, §10 (mail actions), §13, §16; D-007, D-034, D-052, D-053, D-058, D-063
- `crates/fa-mail/src/lib.rs` and `error.rs` (W0 skeleton: `MailDeps`, `MailConfig`; ask the main session for changes), `crates/fa-connectors` (T-02's `gmail` and `imap`), `crates/fa-calendar/src/service.rs` and `jobs.rs` (source CRUD, `sync_due`, secrets and token patterns to copy), `crates/fa-store` (T-03 repos)
- T-10 works in parallel in `process/**`, `inbox.rs`, `labelling.rs`, `setup.rs`, `context.rs`: don't touch those

## You own
- `crates/fa-mail/src/sync/**`
- `crates/fa-mail/src/sync.rs`
- `crates/fa-mail/src/sources.rs`
- `crates/fa-mail/src/actions.rs`
- `crates/fa-mail/src/retention.rs`
- `crates/fa-mail/src/provider.rs`
- `crates/fa-mail/tests/sync_*.rs`
- `crates/fa-store/src/repo/ext_m3_t09.rs`

## Work
1. `provider.rs`: `#[async_trait] trait MailProvider { async fn initial(&mut self, since: NaiveDate, query: Option<&str>) -> Result<Vec<RemoteRef>>; async fn incremental(&mut self) -> Result<Delta>; async fn fetch(&mut self, r: &RemoteRef) -> Result<Fetched{remote_id, thread_id?, folder?, labels, raw}>; async fn modify(&mut self, remote_id, folder?, action) -> Result<()>; fn state(&self) -> ProviderState }` with `GmailProvider` (history id) and `ImapProvider` (per-folder UIDVALIDITY/UIDNEXT via `MailFolderStateRepo`, Gmail extension when advertised, default folders per D-053, never `\Junk`/`\Trash`).
2. `sources.rs` (`SourceService`): list/create/update/archive/test_imap/enqueue_sync as in the plan. Visibility and ownership checks are done by fa-api via authz, but the service filters lists by `viewer` (readable or owned). IMAP password → `Secret` (sealed with `deps.key`, like CalDAV passwords in fa-calendar). Gmail: the oauth account must be owned by the creating adult and its scopes include `https://www.googleapis.com/auth/gmail.readonly` or `gmail.modify` (else `MailError::Invalid("reconnect Google with Gmail access")`). `read_only = false` on a Gmail source requires `gmail.modify`.
3. `sync.rs`: jobs `mail.sync_due` (cron) and `mail.sync{source}`: `mail_defaults::ensure`; provider by kind; first sync vs incremental; 404 history → full backfill; UIDVALIDITY change → reset; for each new message: `fa_rules::parse`, `MessageRepo::insert_parsed` (`pending`; Gmail `remote_id` = id, IMAP Gmail = hex `X-GM-MSGID`, else `uidvalidity:uid`; the external url is computed with `fa_domain::mail::message_external_url`, not stored), enqueue `mail.process{message}` (idempotency `mail.process:{message}`); label changes → `set_labels` and, for SPAM/TRASH moves, re-enqueue `mail.process` with `{message, only: "email.category"}`; errors per Ingest.md §4.7 (`needs_reauth` + `source_reauth` notification rows to owners via `insert_for_adults` or a per-owner insert; key `reauth:mail:{source}:{last_synced_at}`); `set_sync_state`; publish `mail.updated{source}`. Fetch with ≤ 4 in flight. Large first syncs: cap 2,000 messages per job run and re-enqueue the rest.
4. `actions.rs` (`ActionService`): `apply` (the source must not be read-only; Gmail: archive = remove `INBOX`, mark read = remove `UNREAD`, label = add the `FA/…` label id (create on demand, cache per source in memory)); IMAP per D-052), record `mail_action`; `undo` within 30 days (inverse operation); job `mail.auto_actions{message}` applies the message's category `auto_actions` with `automatic = true` (labels `FA/<category label>` and `FA/<member display name>` for linked members). A provider error → `error` on the action row, job retries.
5. `retention.rs`: job `mail.retention` (D-034 as in Domain.md *Message lifecycle*): pending older than `mail_max_pending_days` → `expired`; then the content policy (`purge_after_processing`, `keep_for_learning` keeping user-labelled, eval-sample and the newest `examples_per_category` per category, `keep_all`); purge clears body/html/snippet/headers/attachments text and sets `content_purged_at`.
6. `sync::handlers`/`default_schedules` return your handlers and crons (`mail.sync_due` `* * * * *`, `mail.retention` `30 3 * * *`).
7. **Tests:** wiremock Gmail (first sync, incremental, 404 fallback, label change, `invalid_grant` → needs_reauth via a fake `AccessTokenProvider` returning an error) and greenmail IMAP (gated by `FA_TEST_GREENMAIL` like T-02; first sync, incremental, UIDVALIDITY reset by deleting and recreating the folder, a move action and its undo). Retention policy tests with `#[sqlx::test]`.

## Acceptance
- [ ] `cargo test -p fa-mail` passes, including the greenmail tests run once by you (quote the output)
- [ ] `SQLX_OFFLINE=false cargo build --workspace` passes against the dev DB (the main session re-prepares `.sqlx`)
- [ ] `cargo clippy -p fa-mail --all-targets -- -D warnings` and `cargo fmt --check` are clean (T-10 works in the same crate: if its half doesn't compile yet, report rather than editing it)

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
