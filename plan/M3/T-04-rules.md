---
id: T-04
title: "fa-rules: parse, signals, rules, sections, extractors"
milestone: M3
wave: 2
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-rules/**"]
---

# T-04: fa-rules: parse, signals, rules, sections, extractors

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Rules* (the public API), *Domain types*
- `arch/Ingest.md` §4.8 (content kept), §6 (signals table with confidences, conditions, relevance), §7 (extractors and confidences), §8.1 (dedupe key), §16; decisions D-054, D-055, D-056, D-059, D-060 in `decisions/`
- `crates/fa-domain` as finished by T-01 (`RuleCondition`, `RuleField`, `RuleOp`, `Candidate`, `DecisionPointKey`, `Gender`, `Relevance`)
- Crate docs on docs.rs: `mail-parser 1.0`, `html2text 0.17`, `scraper 0.29`, `pdf-extract 0.12`, `regex 1.13` (`RegexBuilder::size_limit`), `aho-corasick 1.1`, `strsim 0.11`, `icalendar 0.17`

## You own
- `crates/fa-rules/**`

## Work
A pure crate (no DB, no network). The crate skeleton and its `Cargo.toml` exist (W0); you may add workspace deps to it.
1. `parse(raw) -> ParsedMessage`: from, to, cc, subject, date, message-id, thread hints (`In-Reply-To`, `References`), `body_text` (the `text/plain` part, else `html2text` of the HTML at width 100), `body_html` (sanitised: keep the markup, it is only parsed, never rendered as HTML by the app), the header subset of Ingest.md §4.8, attachments (`part`, filename, mime, size, `ics` text for `text/calendar` parts and `.ics` files, `pdf` bytes for PDFs ≤ 5 MB kept in memory only). Malformed mail → best effort, never panic (fuzz a few broken inputs in tests).
2. `signals()` and `evaluate(dp, facts, ctx)`: user rules first by `priority` (lowest number first; the first matching rule for the dp wins with its `value` and `confidence`), then every enabled built-in signal from Ingest.md §6.2 (keys in snake_case; skip those in `ctx.disabled_signals`) plus the D-059 "payment received → is_bill false" and the D-055 tag/gender member signal. Category signals map org kinds to category keys (`school`→`school`, `daycare`→`daycare`, `club`→`activities`, `biller`→`bills`, `other`→ none). Only return category values that exist in `ctx.categories`. Conditions: the fields and ops of §6.1; `contains_any` is whole-word and case-insensitive (`aho-corasick` with word-boundary check); `matches` uses `RegexBuilder` with `size_limit(1 << 20)` and a 256-char pattern cap. `validate_rule` reports the first bad condition with its index.
3. Member linking for `email.member_link`: one candidate per member with the confidences of §6.2; grade phrases cover "Grade 3", "3rd grade", "Year 3", "Kindergarten"/"K", "Pre-K", and the enrollment's class name; names and aliases are whole-word; gender terms come from `ctx.gender_terms` for the member's gender.
4. `sections(facts)`: split HTML on headings (`h1`–`h4`), `hr`, and bold-only paragraphs; plain text on blank-line blocks whose first line is short (≤ 60 chars) and Title Case or ends with ':'. `relevance(section, member, ctx)` per Ingest.md §6.3 (Relevant / Other / General).
5. Extractors (`extract_events`, `extract_bills`, `pdf_text`, `extract_actions`, `contact_hint`) per Ingest.md §7 with the listed confidences. ICS: every VEVENT (start/end with TZID resolution, all-day, location, UID; `METHOD:CANCEL` or `STATUS:CANCELLED` → `cancel = true`). schema.org: JSON-LD `<script type="application/ld+json">` and microdata (`itemtype`) for the listed types. Date phrases: weekday + month + day (+ time or time range), numeric `M/D` (US) **and** `D/M` only when unambiguous (day > 12), "tomorrow/today at 3", "next Friday"; resolved in `ctx.tz` against `received_at` (a date earlier than received → next year if within 2 months of the year end, else drop); title = the subject, or the sentence around the phrase trimmed to 80 chars. Bills: biller template regexes from the org, else the generic parser (currency symbols `$ £ € ¥` and ISO codes; amount near "amount due/total due/balance/payment due"; due date near "due/by"; account hint = last 4 digits after "account"); `pdf_text` via `pdf-extract`. Actions: the phrases of §7 plus "please reply", "let me know by"; a deadline in the same sentence becomes `due_at`; contact questions → `Reply` kind.
6. `event_dedupe_key(title, start, all_day, tz)`: lowercased alphanumeric words of the title joined by '-', the local date, and for timed events the local start rounded down to 15 minutes. `title_similarity` = Jaro-Winkler on the same normalisation.
7. **Tests:** fixture emails under `tests/fixtures/` (all fictional, `example.edu`/`example.com`): a teacher's mail with an ICS invite, a cancellation ICS, a booking with JSON-LD, a school newsletter with grade sections and a "girls' volleyball" section, a utility bill with an amount and due date, a bill whose amount is only in a small generated PDF (create the PDF in the test with a minimal hand-written PDF string, or skip with a clear note if pdf-extract can't read it), a payment-received mail, a promo with `List-Unsubscribe`, a permission-slip mail with "return by Friday". Table tests for date phrases (including DST days in America/New_York and Europe/London) and for every signal.

## Acceptance
- [ ] `cargo test -p fa-rules` passes
- [ ] `SQLX_OFFLINE=true cargo build --workspace` passes
- [ ] `cargo clippy -p fa-rules --all-targets -- -D warnings` and `cargo fmt --check` are clean

## Wave 1 outcomes (read before starting)
- T-01 is done (`cargo test -p fa-domain`: 247 passed). New files `fa-domain/src/{mail,mail_api,pipeline}.rs`. `DecisionPointKey` serialises as the dotted key and has `default_value()`. Settings structs `GenderTerms`, `EventsSettings`, `InboxSettings`, `MailRetention` have `Default` matching the migration; `DEFAULT_CURRENCY`, `DEFAULT_BILL_LEAD_DAYS`. `Message.attachments`/`headers` are `serde_json::Value` (typed element: `AttachmentMeta`). `Resource::MailSource{owners, visibility}`; new actions `ReadMail`, `ManageMailSource`, `ManageMailSetup`, `ReadInbox`, `ManageBills`.
- **Interim defaults** (marked with comments) exist in `fa-store/src/repo/household.rs` and `member.rs` row mappings, `fa-notify/src/service.rs` (`approval_mode`), `fa-api/src/feeds.rs` (`DashboardView.inbox`), `fa-config/src/household.rs`. Replace the ones in your paths.
- T-02 is done: `fa_connectors::google::gmail::*`, `fa_connectors::imap::{ImapClient, ImapConfig, …}` (`internal_date: Option<…>`; `uid_store_flags`/`gm_labels(uid, add: bool, &[&str])`; `capabilities()` is async), `google::calendar::{insert_event, patch_event, delete_event}` returning `Written{id, ical_uid, etag, html_link?}`, `CaldavClient::{put_event, delete_event}`, `ics::build_vevent`, `EventWrite{start, end: DateTime<Utc>, all_day, tz}`. Errors `HistoryExpired`, `InsufficientScope`, `Retryable(u16)`, `AuthFailed`, `Imap(String)`. A `Plain` IMAP connection is made whenever the caller asks for it: callers enforce `allow_plain_imap`.
