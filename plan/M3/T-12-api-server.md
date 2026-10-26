---
id: T-12
title: "fa-api + fa-server + fa-auth: routes, SSE, wiring, CLI, scopes"
milestone: M3
wave: 5
depends_on: [T-05, T-06, T-07, T-08, T-09, T-10, T-11]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-api/**", "bins/fa-server/**", "crates/fa-auth/**", "crates/fa-calendar/**", "crates/fa-store/src/repo/ext_m3_t12.rs"]
---

# T-12: fa-api + fa-server + fa-auth: routes, SSE, wiring, CLI, scopes

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *HTTP API v1 additions*, *Domain types*, *Process config*, *Jobs*, *Mail service*, *fa-calendar additions*
- `arch/Ingest.md` §11, §12; D-063, D-019
- `crates/fa-api/src` (routes.rs, tasks.rs and feeds.rs handler patterns, openapi.rs, hub.rs SSE kinds, error.rs), `bins/fa-server` (wiring of services, job handlers and schedules, CLI subcommands), `crates/fa-auth/src/oauth.rs` (`SCOPES`, `start`)
- The Wave 3–4 reports in *Wave 4 outcomes* below (filled in by the main session)

## You own
- `crates/fa-api/**`
- `bins/fa-server/**`
- `crates/fa-auth/**`
- `crates/fa-store/src/repo/ext_m3_t12.rs`
- `crates/fa-calendar/**` (for the small gaps listed under *Wave 4 outcomes*; T-11 is done)

## Work
1. **fa-auth:** `OAuthService::start(…, purpose)` requests the base scopes plus the purpose's scope (`gmail_read` → `https://www.googleapis.com/auth/gmail.readonly`, `gmail_modify` → `https://www.googleapis.com/auth/gmail.modify`, `calendar_write` → `https://www.googleapis.com/auth/calendar.events`) with `include_granted_scopes=true`; the callback stores the union of granted scopes on the existing account (same email) instead of failing. Tests.
2. **fa-api:** every route in the plan's table with utoipa annotations, authorisation via `fa_domain::authorize` (resource `MailSource{owners, visibility}` loaded per message/source), 404 for messages the viewer can't read (don't leak existence), If-Match on versioned PATCHes. Inbox approve/reject dispatch: event → `CalendarService`, bill/contact → `InboxService`, task → `TaskService::accept` (with `TaskApproval` edits applied first via `update_task`) / `skip`. Devices: `GET /inbox` returns counts only unless elevated. `GET /dashboard` gains `inbox` counts. SSE kinds `mail.updated`, `inbox.updated`, `bills.updated`. Audit log entries (D-013) for approvals, rejections, mail actions and undo.
3. **fa-server:** build `MailDeps` (token provider = `OAuthService`, `MailConfig` from `ProcessConfig`), pass `google_api_base` to the calendar remote too, register fa-mail handlers and schedules, the new fa-calendar/fa-tasks handlers, `gen-openapi` includes the new paths; CLI `mail sync-now [--source NAME]` and `mail process-now`.
4. Tests: route tests in the existing style for visibility (owner vs other adult vs household source vs child vs device), inbox approve/reject per kind, device counts-only, mail action on a read-only source → 409, undo after 30 days → 410, rule validation → 422.

## Acceptance
- [ ] `cargo test -p fa-api -p fa-auth -p fa-server` passes
- [ ] `SQLX_OFFLINE=false cargo build --workspace` passes against the dev DB (the main session re-prepares `.sqlx`)
- [ ] `cargo clippy --workspace --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] `cargo run -p fa-server -- gen-openapi --out /tmp/claude-openapi-check.json` works (don't overwrite `apps/web/src/gen`)

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

## Wave 3 outcomes (T-06, T-07, T-08)
- **T-06 (fa-tasks):** `TaskService::from_mail(MailTaskRequest)` and `from_bill`; jobs `tasks.from_mail` / `tasks.from_bill` accept an optional `household` (else looked up from the message/bill). `complete`/`reopen` flip the bill and publish `bills.updated`. Digest adds "N items waiting for approval".
- **T-07 (fa-notify):** `approval_mode` honoured in `notify.send`; new adult prefs enable `approval_needed` and `mail_time_sensitive`. Main session added migration `20261027000002_m3_adult_mail_triggers` (backfills those triggers for existing adults) and a batch click path of `/inbox` when a batch is only approval/mail rows.
- **T-08 (fa-config):** `ProcessConfig.{allow_plain_imap, imap_tls_insecure, google_api_base: Option<Url>}` (env `FA_ALLOW_PLAIN_IMAP`, `FA_IMAP_TLS_INSECURE`, `FA_GOOGLE_API_BASE`): **fa-server must pass them into `MailConfig` and the calendar remote config**. `ImportReport` gained `orgs_*`, `enrollments_*`, `mail_sources_*`, `categories_*`, `mail_rules_*` lists and `mail_sources_skipped` (Gmail sources can only be created through OAuth in the UI): print the skipped ones in the CLI as "connect these in Settings → Mail". Import now switches its transaction to the household scope (fixed a cross-household read bug).

## Wave 4 outcomes: T-11 (fa-calendar)
- `CalendarService::{route_event, proposals, approve_event(household, id, EventApproval, by), reject_event, approve_cancel, reject_cancel, write_event, delete_event}`; job payloads `{household, event}`. State conflicts return `CalendarError::Invalid` (422 today): **map approve/reject of an event that is not in the right state to 409** (add a variant and map it).
- Google written events keep the Google event `id` as `remote_uid` (the sync keys by id); `iCalUID` is in `source_ref.ical_uid`.
- **Gaps for you to close (you own `crates/fa-calendar/**` now):**
  1. Nothing sets `calendar_source.writable` or `calendar.is_default_target`. Add `writable` to the calendar source view and PATCH (Google sources: require the account's scopes to include `https://www.googleapis.com/auth/calendar.events`, else 422 "reconnect with calendar write access") and `is_default_target` to the calendar view and PATCH (at most one default per member set is fine to leave unenforced). T-14 adds the toggles.
  2. `proposals` should also list `approved` events with a `write_error` (so the inbox shows the failure and lets the adult retry by approving again, or reject).

## Wave 4 outcomes: T-10 (fa-mail processing side)
- Services: `fa_mail::context::Viewer{member, role, device, elevated}`; `ProcessService` (job `mail.process{message, household?, only?}`), `InboxService::{list, list_kind, counts, approve_bill, reject_bill(BillPatch status dismissed routes here), approve_contact, reject_contact}` (devices without elevation get counts only; children `Forbidden`), `LabelService::{next, label(household, viewer, message, input), set_eval_sample, progress}` (`EVAL_TARGET` 200), `SetupService` (orgs, contacts, enrollments, rules + `test_rule`, signals, categories, `make_task`, `list_messages`, `message_detail`; PATCHes take `version: i32` from If-Match).
- **PDF bytes:** `ProcessService::with_attachments(Arc<dyn AttachmentSource>)` is needed to fetch PDF bytes for bills; the default `fa_mail::handlers()` builds `ProcessService` without one. Wire an `AttachmentSource` in fa-server that re-fetches the raw message through the source's provider (T-09's provider API) and returns the PDF part via `fa_rules::parse`. If T-09's API makes this awkward, report it rather than duplicating provider code.
- `MakeTaskInput.list` is ignored (the `tasks.from_mail` payload has no list).

## Wave 4 outcomes: T-09 (fa-mail sync side)
- `SourceService::{list, create, update(household, viewer, id, patch, expected_version), archive, test_imap, enqueue_sync}`, `SyncService` (jobs `mail.sync_due`, `mail.sync{household, source}`), `ActionService::{apply, undo, auto_actions}` + `actions::view`, `actions::fa_label`, `retention::{run_household, run_all}`. `sync::handlers` also returns `mail.auto_actions` and `mail.retention`. Build these on `Arc<MailDeps>`; `fa_mail::handlers(&deps)` registers everything.
- `provider::open(...)` builds a provider for a source; `fetch_raw(&Target{remote_id, folder, rfc822_id})` re-reads a raw message: **use it for the `AttachmentSource` (PDF bytes) wiring** described under T-10.
- Transient sync errors keep `state = ok` with `last_error`; permanent misconfiguration sets `error`. IMAP security by port: 993 implicit TLS, else STARTTLS, or plain only with `allow_plain_imap`.
- `.sqlx` re-prepared and the backfill migration applied to the dev DB by the main session after Wave 4; the workspace builds.
