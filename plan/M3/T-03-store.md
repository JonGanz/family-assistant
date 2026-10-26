---
id: T-03
title: "fa-store: M3 migration, repos, seeding, .sqlx"
milestone: M3
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-store/**", ".sqlx/**"]
---

# T-03: fa-store: M3 migration, repos, seeding, .sqlx

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Migration* (exact tables and columns), *Seeding*, *Pipeline engine*, *Mail service*, *fa-calendar additions*, *fa-tasks additions*, *fa-notify additions*, *Jobs*
- `arch/Ingest.md` §3, §8; `arch/Domain.md` *Persistence conventions*
- `crates/fa-store`: migrations `20261024000002_identity.up.sql` (RLS, grants, triggers) and `20261026000001_m2_tasks.up.sql` (latest style); `store.rs`; `repo/*` (patterns, `Versioned`, `conflict_or_missing`); `repo/event.rs` (`upsert_synced`, `delete_unseen_in_window`); `repo/notification.rs`
- `crates/fa-domain` as finished by T-01

## You own
- `crates/fa-store/**`
- `.sqlx/**`

## Work
1. **Migration** `migrations/20261027000001_m3_ingest.{up,down}.sql` with exactly the plan's table, including the CHECK replacements on `event.status`, `task_list.kind`, `task.origin`, `notification.trigger`, and the seed inserts for existing households. RLS + FORCE + `tenant` policy + `set_updated_at` + grants on every new table. Down drops everything in reverse and restores the old CHECKs. Apply it to the dev DB (schema only).
2. **Replace interim defaults** T-01 put into fa-store row mappings with the real columns (household, member, notify_pref, event, task_list, task).
3. **Repositories** (associated fns on `&mut PgConnection`; versioned rows return `Versioned<T>`; version mismatch → `StoreError::Conflict`). Wave 3–4 rely on these names; add more if useful:
   - `mail_defaults::ensure(conn, household)` (the seed; idempotent).
   - `OrgRepo` insert/get/list(include_archived)/update/archive/get_by_name; `OrgContactRepo` insert/list(org)/update/archive/by_email(email) → `Vec<(OrgContact, OrgId)>`; `EnrollmentRepo` insert/list(member: Option)/update/delete/set_contacts/contacts(enrollment); `ContactSuggestionRepo` insert_if_new (false when the (org, email) exists in any status)/list(status)/set_status/expire_older_than(cutoff) -> u64.
   - `MailSourceRepo` insert(&new, owners)/get/list(include_archived) with owners/update/archive/owners(id)/due_for_sync(now) -> `Vec<(HouseholdId, MailSourceId)>` (system tx: not archived, state `ok`, `last_synced_at IS NULL OR last_synced_at + poll_minutes <= now`)/set_sync_state(id, history_id?, state, last_error?, synced_at?)/set_gmail_ext/readable_by(member) -> ids (owner or `household` visibility); `MailFolderStateRepo` get/upsert/reset.
   - `MessageRepo` insert_parsed(&NewMessage) -> Option<MessageId> (`ON CONFLICT (source_id, remote_id) DO NOTHING`)/get/get_with_content/list(&MessageFilter{sources: &[MailSourceId], category, member, state, q (subject/from ILIKE), include_ignored_keys: Option<&[String]>, before, limit})/set_labels(id, gmail_labels)/set_classification(id, category, time_sensitive, is_bill)/set_members(id, &[(MemberId, DecidedBy, f32)])/members(id)/set_state(id, state)/set_eval_sample/set_learning_example/attachments_update(id, jsonb)/purge_content(ids)/retention_candidates(policy...)/pending_older_than(cutoff)/label_queue(household, limit) (stratified: round-robin over categories, preferring messages whose latest decisions are non-final, excluding those already labelled by a user)/eval_progress -> (in_sample, labelled).
   - `MessageMemberRepo` if separate. `DecisionPointRepo` get_by_key; `PipelineRepo` active_version(dp) -> (PipelineVersion, Vec<PipelineStage>)/versions(dp)/insert_version; `DecisionRepo` insert/latest(dp, subject)/for_subject(subject)/delete_for_subjects(type, ids).
   - `MailRuleRepo` insert/get/list(dp: Option, include_archived)/update/archive; `CategoryRepo` insert/list(include_archived)/get/get_by_key/update; household `disabled_signals` read/write via `HouseholdRepo`.
   - `BillRepo` insert_if_new(&new) -> Option<Versioned<Bill>> (dedupe_key)/get/list(status)/update/set_status/set_task(bill, task)/by_task(task)/set_paid_by_task(task) -> bool/reopen_by_task(task) -> bool/for_message(message).
   - `MailActionRepo` insert/get/list_for_message/mark_undone/set_error.
   - `EventRepo` (extend): insert_extracted(&NewExtractedEvent{.., members}) -> EventId (dedupe_key, message_id, org_id, cancelled flag)/get_full/dedupe_candidates(household, start_from, start_to) -> rows (any origin; excluding `rejected`/`expired`)/set_route(id, target, route_confidence)/set_status(id, status, by?)/set_written(id, calendar, remote_uid, etag, external_url)/set_write_error/set_pending_cancel/proposals() -> rows with message subject/from and org name/link_message(event, message)/delete(id)/expire_past(now) -> u64/derived_for_message(message). **Fix `upsert_synced`** so a conflict with a row whose `origin = 'extracted'` leaves its origin, status, source_ref and message link unchanged (only refreshes etag/times). Test it.
   - `TaskRepo` (extend): derived_for_message(message) (source_ref->>'message_id'), suggested_email_older_than(cutoff) -> ids; `TaskListRepo::get_or_create(kind, name, defaults)`.
   - `InboxRepo::counts(conn) -> InboxCounts` (events `proposed` or `pending_cancel`; bills `proposed`; tasks `suggested` with origin `email`; contacts `proposed`).
   - `NotificationRepo::insert_for_adults(conn, trigger, subject_type, subject_id, title, body, click_path, key_prefix)` (one row per non-archived adult, dedupe `{key_prefix}:{member}`).
   - `NotifyPrefRepo`: `approval_mode` in reads and writes.
4. Register empty modules `ext_m3_t05.rs` … `ext_m3_t12.rs` (one per Wave 3–5 task: t05, t06, t07, t08, t09, t10, t11, t12) in `repo/mod.rs` with a one-line comment, for those tasks to fill.
5. **Tests** (`#[sqlx::test]`): migration up and down; RLS isolation on every new table; seed idempotent and complete (12 categories, 5 decision points each with one active 'Rules only' version of two stages); message `(source, remote_id)` dedupe; bill dedupe; `upsert_synced` not clobbering an extracted written row; `due_for_sync`; `label_queue` stratification; `InboxRepo::counts`; `insert_for_adults`; `set_paid_by_task`/`reopen_by_task`; version conflicts.
6. `cargo sqlx prepare --workspace` (with `SQLX_OFFLINE=false` and the dev `DATABASE_URL`) and keep `.sqlx` current.

## Acceptance
- [ ] `cargo test -p fa-store` passes on the dev DB **and** on a fresh throwaway Postgres (port 55403)
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-store --all-targets -- -D warnings` and `cargo fmt --check` are clean

## Wave 1 outcomes (read before starting)
- T-01 is done (`cargo test -p fa-domain`: 247 passed). New files `fa-domain/src/{mail,mail_api,pipeline}.rs`. `DecisionPointKey` serialises as the dotted key and has `default_value()`. Settings structs `GenderTerms`, `EventsSettings`, `InboxSettings`, `MailRetention` have `Default` matching the migration; `DEFAULT_CURRENCY`, `DEFAULT_BILL_LEAD_DAYS`. `Message.attachments`/`headers` are `serde_json::Value` (typed element: `AttachmentMeta`). `Resource::MailSource{owners, visibility}`; new actions `ReadMail`, `ManageMailSource`, `ManageMailSetup`, `ReadInbox`, `ManageBills`.
- **Interim defaults** (marked with comments) exist in `fa-store/src/repo/household.rs` and `member.rs` row mappings, `fa-notify/src/service.rs` (`approval_mode`), `fa-api/src/feeds.rs` (`DashboardView.inbox`), `fa-config/src/household.rs`. Replace the ones in your paths.
- T-02 is done: `fa_connectors::google::gmail::*`, `fa_connectors::imap::{ImapClient, ImapConfig, …}` (`internal_date: Option<…>`; `uid_store_flags`/`gm_labels(uid, add: bool, &[&str])`; `capabilities()` is async), `google::calendar::{insert_event, patch_event, delete_event}` returning `Written{id, ical_uid, etag, html_link?}`, `CaldavClient::{put_event, delete_event}`, `ics::build_vevent`, `EventWrite{start, end: DateTime<Utc>, all_day, tz}`. Errors `HistoryExpired`, `InsufficientScope`, `Retryable(u16)`, `AuthFailed`, `Imap(String)`. A `Plain` IMAP connection is made whenever the caller asks for it: callers enforce `allow_plain_imap`.
