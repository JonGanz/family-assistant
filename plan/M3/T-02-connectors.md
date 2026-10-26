---
id: T-02
title: "fa-connectors: Gmail client, IMAP client, Google/CalDAV writes"
milestone: M3
wave: 1
depends_on: []
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-connectors/**"]
---

# T-02: fa-connectors: Gmail client, IMAP client, Google/CalDAV writes

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Crates* (workspace deps), *Mail service* (what T-09 and T-11 will call), *fa-calendar additions* (writes)
- `arch/Ingest.md` §4 (sync steps, exact Gmail endpoints and history types), §8.4 (writes), §16
- `crates/fa-connectors/src`: `google/calendar.rs` (request style, `Http`, error mapping), `caldav.rs`, `http.rs`, `error.rs`
- Gmail REST reference on developers.google.com: `users.getProfile`, `users.messages.list` (`q`, `pageToken`, `maxResults`), `users.history.list` (`startHistoryId`, `historyTypes`, 404 on an expired id), `users.messages.get` (`format=raw` is base64url), `users.messages.attachments.get`, `users.messages.modify`, `users.labels.list/create`. Google Calendar `events.insert/patch/delete` (`iCalUID` is returned by insert).
- `async-imap 0.12` docs (docs.rs), RFC 9051/3501 (UID FETCH/SEARCH, `BODY.PEEK[]`), RFC 6851 (MOVE), RFC 6154 (special-use), Gmail IMAP extensions (developers.google.com/gmail/imap/imap-extensions: `X-GM-EXT-1`, `X-GM-MSGID`, `X-GM-THRID`, `X-GM-LABELS`, `X-GM-RAW`)

## You own
- `crates/fa-connectors/**`

## Work
Add `crates/fa-connectors` dependencies as `workspace = true`: `async-imap`, `tokio-rustls`, `webpki-roots` (and `base64`, `futures` if needed). No DB access in this crate.
1. **`gmail` module** (`google/gmail.rs` or `gmail.rs`), functions taking `(&Http, base, access_token, …)` like `google/calendar.rs`: `get_profile -> Profile{email, history_id}`, `list_messages(q, page_token) -> (Vec<MsgRef{id, thread_id}>, Option<String>)`, `list_history(start_history_id, page_token) -> Result<HistoryPage{added: Vec<MsgRef>, labels_added: Vec<(String, Vec<String>)>, labels_removed: Vec<(String, Vec<String>)>, deleted: Vec<String>, history_id, next_page_token}>` with a distinct `ConnectorError::HistoryExpired` on 404, `get_raw(id) -> RawMessage{id, thread_id, label_ids, internal_date: DateTime<Utc>, raw: Vec<u8>}` (base64url decode), `get_attachment(message, attachment) -> Vec<u8>`, `modify(id, add: &[String], remove: &[String])`, `list_labels`, `create_label(name) -> Label{id, name}`. 401 → `Unauthorized`, 403 insufficient scope → `InsufficientScope`, 429/5xx → retryable error. Tests with `wiremock` for each, including pagination and the 404.
2. **`imap` module**: `ImapConfig{host, port, security: Implicit|StartTls|Plain, username, password, insecure_tls: bool}`; `ImapClient::connect(cfg) -> Result<ImapClient>` (rustls with `webpki-roots`; `insecure_tls` uses a verifier that accepts any cert — tests only; `Plain` only when the caller allows it); `capabilities() -> Caps{gmail_ext, move_, condstore, uidplus, idle}`; `list_folders() -> Vec<Folder{name, special_use: Option<SpecialUse>}>` (RFC 6154 attributes: `\All`, `\Archive`, `\Junk`, `\Trash`, `\Sent`, `\Drafts`); `select(folder) -> FolderStatus{uidvalidity, uidnext, highestmodseq?}`; `uid_search_since(date: NaiveDate, gm_raw: Option<&str>) -> Vec<u32>`; `uid_fetch(uids, with_gmail: bool) -> Vec<ImapMessage{uid, raw, flags, internal_date, gm_msgid?: u64, gm_thrid?: u64, gm_labels: Vec<String>}>` (batches of 50, `BODY.PEEK[]` so nothing is marked read); `uid_fetch_range(from_uid)` for incremental; `labels_changed_since(modseq) -> Vec<(u32, Vec<String>)>` (Gmail + CONDSTORE); `uid_move(uid, folder)` (MOVE, else COPY + `\Deleted` + UID EXPUNGE when UIDPLUS); `uid_store_flags(uid, add/remove, flags)`; `gm_labels(uid, add/remove, labels)`; `logout()`. Tests against a throwaway `greenmail/standalone:2.1.8` container (plain IMAP on 3143 and IMAPS on 3993 with `insecure_tls`; put mail in with IMAP `APPEND`): connect, select, search since, fetch raw bytes equal to what was appended, incremental by UIDNEXT, move, flags. Mark these tests `#[ignore]` unless `FA_TEST_GREENMAIL=host:imap_port:imaps_port` is set, and give the exact command to run them in a doc comment (container name `fa-t02-greenmail`, removed with `docker rm -f -v`). Run them yourself.
3. **Google Calendar writes** in `google/calendar.rs`: `insert_event(calendar_id, &EventWrite{summary, description?, location?, start, end, all_day, tz}) -> Written{id, ical_uid, etag, html_link?}`, `patch_event`, `delete_event` (410/404 count as done). wiremock tests.
4. **CalDAV writes** in `caldav.rs`: `put_event(calendar_url, uid, ics_text, if_none_match: bool, if_match: Option<&str>) -> Option<String /*etag*/>`, `delete_event(href, etag: Option<&str>)`. A small `ics::build_vevent(uid, &EventWrite) -> String` helper (with `icalendar`). wiremock tests.
5. `pub use` the new types from `lib.rs`. Update `dto.rs`/`error.rs` as needed without breaking existing callers.

## Acceptance
- [ ] `cargo test -p fa-connectors` passes (with the greenmail tests run once by you; quote both outputs)
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-connectors --all-targets -- -D warnings` and `cargo fmt --check` are clean
- [ ] No container or anonymous volume left behind
