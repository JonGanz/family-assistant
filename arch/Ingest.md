---
title: Ingest — email, rules pipeline, bills, linking, approvals
scope: Mail sources (Gmail API and IMAP) and sync, message storage and retention, the decision pipeline engine (rules-only preset), deterministic classifiers and extractors, orgs, contacts and enrollments, bills, event proposals and calendar writes, the approval inbox, email tasks, the labelling view
status: draft
updated: 2026-10-25
drawio: Ingest.drawio
drawio_src: src/Ingest.py
depends_on: [D-001, D-004, D-006, D-007, D-008, D-009, D-010, D-012, D-013, D-014, D-018, D-019, D-023, D-024, D-027, D-028, D-029, D-034, D-035, D-036, D-044, D-045, D-052, D-053, D-054, D-055, D-056, D-057, D-058, D-059, D-060, D-061, D-062, D-063]
milestone: M3
---

# Ingest: email, rules pipeline, bills, linking, approvals (DRAFT v1)

This doc covers M3 (D-023, as adjusted by D-029):
- mail ingest through the Gmail API or IMAP (D-052);
- the decision pipeline engine with the **'Rules only'** preset;
- deterministic classification and extraction;
- org, teacher and child linking;
- bills (D-009);
- event proposals with calendar writes and the approval inbox (D-008);
- tasks from email;
- the labelling view that builds the eval sample (D-028).

v1 is re-derived from PROJECT.md and all accepted decisions, including batch 16 (D-052 to D-063). **In M3 nothing extracted is written to a calendar without approval** (D-057).

**Not in M3:**
- System One, kNN, the LLM (System 2) and their stages; Pipeline and Backends settings screens; `fa-eval` (all M4, `AI.md`);
- learned calendar routing (M6);
- deals (D-012), school portal APIs (D-014), budgeting (D-009).

## 1. Concepts
| Concept | What it is | Decided by |
|---|---|---|
| **Mail source** | A mailbox (Gmail API or IMAP) linked to one or more owner members, read-only by default, visible to its owners or to all adults | D-007, D-052, D-063 |
| **Message** | One email. Its metadata is always kept; its content follows the retention policy | D-034 |
| **Org** | A school, daycare, club, biller or other sender group, with its domains, sender addresses and optional ICS feeds | D-014 |
| **Contact / Enrollment** | A person at an org (a teacher, a coach), and which child is enrolled where (grade, class). This is how a teacher's email reaches the right child. New contacts are suggested | D-056 |
| **Decision point** | A question asked about a subject: `email.category`, `email.time_sensitive`, `email.member_link`, `email.is_bill`, `event.calendar_route`, `task.urgency` (M4) | D-027 |
| **Pipeline** | A versioned, ordered list of stages for one decision point. M3 runs `rules` and `human` stages only | D-029 |
| **Extractor** | A deterministic producer of candidate events, bills and tasks from a message (ICS parts, schema.org data, biller templates, date phrases) | D-001, D-009 |
| **Proposal** | An extracted event (always, in M3), a bill below threshold, a suggested task or a suggested contact, waiting in the approval inbox | D-008, D-056, D-057, D-060 |
| **Labelling view** | Where adults confirm or correct the decisions on a message and build the ~200-message eval sample | D-028 |

## 2. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-connectors` (extended) | `gmail` client: `users.getProfile`, `messages.list` (with `q`), `history.list`, `messages.get` (`format=raw`), `attachments.get`, `messages.modify`, `labels.list/create`. `gcal` writes (`events.insert/patch/delete`) and CalDAV `PUT`/`DELETE` of a VEVENT. `imap`: a provider-neutral IMAP client (TLS login, folder `SELECT`, UID `FETCH`/`SEARCH`, `MOVE`, `STORE` flags, and the Gmail `X-GM-*` extension when advertised). No DB access | (external crates only) |
| `fa-pipeline` (new) | The engine (§5): loads a decision point's active `PipelineVersion`, runs the enabled stages in order with thresholds and `on_unavailable`, and records `Decision` rows. The `Stage` trait has the `rules` and `human` implementations in M3; `knn`, `system_one` and `llm` are M4 and are treated as unavailable. Built-in presets, with 'Rules only' active | fa-domain, fa-store |
| `fa-rules` (new) | Deterministic rulesets and extractors, as pure functions over a parsed message (§6, §7): header and Gmail-label signals, org and contact matching, member name and grade matching, bill detection and biller templates, the date-phrase finder, ICS and schema.org extraction, PDF text | fa-domain (+ `mail-parser`, `html2text`, `scraper`, `pdf-extract`, `regex`, `aho-corasick`) |
| `fa-mail` (new) | Sources and sync for both providers behind one `MailProvider` trait (§4), message storage, processing (`mail.process`: runs the email pipelines, then the extractors, then emits proposals through the store and job queue), Gmail actions with undo, the retention job, the labelling view's service | fa-domain, fa-store, fa-jobs, fa-connectors, fa-pipeline, fa-rules |
| `fa-calendar` (extended) | `calendar.route`: dedupe, the `event.calendar_route` pipeline, the confidence gate, then either writes the event to Google or CalDAV or leaves it `proposed` for the inbox. Approve, edit and reject | + fa-pipeline |
| `fa-tasks` (extended) | `tasks.from_mail` (a suggested or created email task, D-060) and `tasks.from_bill` (a "Pay <payee>" pool task over the adults, D-059). Completing the pay task marks the bill paid | (unchanged) |
| `fa-notify` (extended) | New trigger kinds: `mail_time_sensitive` and `approval_needed` | (unchanged) |
| `fa-api` (extended) | Routes (§11), SSE kinds `mail.updated` and `inbox.updated` | + fa-mail |

The coupling rule from M1 and M2 holds: **feature crates never depend on each other.** `fa-mail` emits through the store and the job queue:
- it inserts an `event` row with `status = 'extracted'`, then enqueues `calendar.route{event}`;
- it inserts a `bill` row, then enqueues `tasks.from_bill{bill}`;
- it enqueues `tasks.from_mail{message, kind}`;
- it inserts `notification` outbox rows.

`fa-pipeline` and `fa-rules` are shared infrastructure, like `fa-jobs`: any feature crate may use them.

**External crates** (checked on crates.io on 2026-10-25): `mail-parser 1.0` (MIME), `html2text 0.17`, `scraper 0.29` (schema.org JSON-LD and microdata), `pdf-extract 0.12` (D-009), `regex 1.13`, `aho-corasick 1.1` (many keyword and name matches), `strsim 0.11` (dedupe title similarity), and `icalendar 0.17` (already used), `async-imap 0.12` (IMAP over tokio with rustls). Date phrases are found by our own regex patterns and resolved with `chrono` against the message's received time and the household time zone. The natural-language date crates found (`chrono-english`, `interim`, `two_timer`) parse a whole string and cannot find dates inside free text.

## 3. Data model (page "Model")
This refines the Organizations & Mail and AI & Pipelines packages of [Domain.md](Domain.md). All tables are household-scoped with RLS.

| Table | Key columns | Notes |
|---|---|---|
| `org` | kind: `school\|daycare\|club\|biller\|other`, name, domains[], senders[], calendar_source? (ICS feed, D-014), default_calendar?, archived_at?, version | `domains` match the sender's domain or subdomains. `senders` are exact addresses |
| `org_contact` | org, name, email, role: `teacher\|coach\|admin\|office\|other`, archived_at? | Unique `(org, lower(email))` |
| `enrollment` | member, org, grade?, class?, from?, until? | A child at a school, daycare or club |
| `enrollment_contact` | enrollment ↔ org_contact | This child's teacher or coach |
| `org` (+col) | contact_mode: `suggest\|auto\|manual` (default `suggest`) | D-056 |
| `contact_suggestion` | org, email, name?, member?, message, status: `proposed\|accepted\|rejected\|expired` | Unique `(org, lower(email))`, so a rejected address isn't proposed again |
| `member` (+cols) | `aliases[]` (nicknames to match in mail), `gender?`: `female\|male\|other`, `relevance_tags[]` | D-055. Used by member linking (§6.3) |
| `household` (+settings) | `gender_terms{female[], male[]}`, `events.auto_write` (false), `events.thresholds{extract: 0.9, route: 0.9}`, `inbox.expiry{task_days: 14, contact_days: 14}` | D-055, D-057, D-061 |
| `mail_source` | provider: `gmail\|imap` (default `gmail`), address, read_only (true), visibility: `owners\|household` (default `owners`), backfill_days (30), query? (Gmail search string), poll_minutes (5), state: `ok\|needs_reauth\|error\|paused`, last_synced_at?, last_error?, archived_at?, version. **Gmail:** oauth_account, history_id?. **IMAP:** host, port (993), username, secret (app password), folders[] (default `INBOX`; `[Gmail]/All Mail` on Gmail), archive_folder?, gmail_ext (detected) | D-007, D-052, D-053, D-063. Gmail reuses `oauth_account` (Feeds.md §2) with incremental consent for `gmail.readonly`, or `gmail.modify` when read-only is off |
| `mail_folder_state` | source, folder, uidvalidity, uidnext | IMAP incremental sync state |
| `mail_source_owner` | mail_source ↔ member (1..*) | The owners are the classification context, and the visibility boundary unless the source is `household` (D-063) |
| `message` | source, remote_id (Gmail id, hex `X-GM-MSGID`, or `uidvalidity:uid`), folder?, thread_id?, rfc822_id?, received_at, from_addr, from_name?, to[], cc[], subject, gmail_labels[], has_attachments, category?, time_sensitive?, is_bill?, processing_state: `pending\|processed\|expired`, **content** (body_text?, snippet?, headers jsonb?, attachments jsonb?), is_learning_example, content_purged_at?, in_eval_sample | Unique `(source, remote_id)`. Mail sent to several of the household's sources is one message per source; the derived items are deduped (§8) |
| `message_member` | message ↔ member, decided_by: `rule\|user` | Result of `email.member_link`, which can give several members |
| `decision_point`, `pipeline_version`, `stage`, `decision` | As in Domain.md (AI & Pipelines) | Created in M3, so M4 only adds stage kinds |
| `mail_rule` | decision_point, name, priority, conditions jsonb (§6.1), value, confidence, enabled, origin: `builtin\|user`, version | User rules for the `rules` stage. Built-in rules are code with listed defaults, and can be disabled |
| `category` | key, label, behaviour: `ignore\|keep\|extract`, notify_time_sensitive, auto_actions[] (`label` default; `archive`, `mark_read` optional), email_tasks: `suggest\|create\|off` (default `suggest`), color, ord, archived_at? | Seeded with the D-054 set; D-058, D-060 |
| `bill` | message?, org?, payee, amount_minor, currency, due_on?, account_hint?, statement_period?, status: `proposed\|open\|paid\|dismissed`, confidence, task?, source_ref, version | D-009, D-059 (no autopay). `proposed` waits in the inbox |
| `event` (+use) | origin `extracted`, status `extracted\|proposed\|approved\|written\|rejected\|expired`, confidence, source_ref, dedupe_key, target_calendar? | Columns from M1 (Feeds.md §2), now used |
| `task` (+use) | origin `email`, status `suggested` (D-060), source_ref, list kind `email` | Domain.md Tasks package |
| `gmail_action` | message, action: `archive\|mark_read\|label`, label?, automatic, done_by?, done_at, undone_at? | D-007, D-058. Undoable for 30 days. Named for D-007; also used for IMAP sources |

`SourceRef{kind: email, message_id, external_url}` links every derived item back to its message. `external_url` is the Gmail deep link `https://mail.google.com/mail/u/?authuser=<address>#all/<id>` for Gmail API and Gmail IMAP sources (the hex `X-GM-MSGID` is the same id). Other IMAP messages have no external link; the app's message view stands in.

## 4. Mail sync (D-007, D-052, D-053)
Both providers implement `MailProvider { initial(window, filter), incremental(state), fetch(id) -> raw RFC 822, modify(id, action) }`. Everything after the raw message is shared.

1. **Connect:** Settings → Mail → "Add mailbox", choosing **Gmail (Google sign-in)** (default) or **IMAP (app password)**.
   - Gmail reuses the Google OAuth flow from M1, with incremental consent for `https://www.googleapis.com/auth/gmail.readonly`, or `gmail.modify` when read-only is off. Both are **restricted scopes**, so with the household's OAuth client in Testing mode refresh tokens expire after 7 days.
   - IMAP takes host, port, username and an app password (stored as a `Secret`). The connect step logs in, lists folders, and detects `X-GM-EXT-1`. For a Gmail address it pre-fills `imap.gmail.com:993` and `[Gmail]/All Mail`.
2. **Owners, visibility and filter:** the adult picks the owner members, `owners` or `household` visibility (D-063), the backfill window, and the optional filter (D-053): a Gmail search string, and for IMAP the folder list. The default reads everything except Spam and Trash.
3. **Scheduling:** `mail.sync_due` runs every minute and enqueues `mail.sync{source}` when `last_synced_at + poll_minutes` has passed. The idempotency key is `mail.sync:{source}:{slot}`. Gmail push (Pub/Sub) needs a public endpoint and IMAP IDLE needs a held connection per mailbox, so M3 polls.
4. **Gmail API sync:**
   - first: read the `historyId` from `users.getProfile`, page through `messages.list?q=newer_than:{backfill_days}d {query}`, and fetch each new message with `messages.get?format=raw` (up to 4 in flight);
   - incremental: `history.list?startHistoryId=…&historyTypes=messageAdded&historyTypes=labelAdded&historyTypes=labelRemoved`. Label changes update `gmail_labels`; moving a message to Spam or Trash re-runs only `email.category`.
   - A **404** means the history ID is too old (kept at least a week, sometimes only hours). The source falls back to a full sync of the backfill window, which is safe because `(source, remote_id)` is unique.
5. **IMAP sync**, per folder:
   - first: `SELECT`, store `UIDVALIDITY` and `UIDNEXT`, `UID SEARCH SINCE <backfill date>` (with `X-GM-RAW "<query>"` when Gmail-extended and a query is set), then `UID FETCH` of `BODY.PEEK[]` (plus `X-GM-MSGID X-GM-THRID X-GM-LABELS` when extended) in batches of 50;
   - incremental: `UID FETCH <uidnext>:*`. Gmail-extended sources also refresh labels for the last `backfill_days` with `CHANGEDSINCE` when `CONDSTORE` is offered, or skip label refresh otherwise;
   - a changed `UIDVALIDITY` resets the folder and refetches the backfill window; uniqueness on `remote_id` (which is `X-GM-MSGID` on Gmail) keeps it idempotent. On non-Gmail servers, the same message in two configured folders is two rows, so the default is `INBOX` only.
   - `\Junk`/`\Trash` special-use folders are never read.
6. **Then, for both:** parse with `mail-parser`, store `pending`, enqueue `mail.process{message}`.
7. **Errors:** Gmail `invalid_grant` or an IMAP `AUTHENTICATIONFAILED` sets the account or source to `needs_reauth`, pauses it, and sends a `source_reauth` notification (M2 trigger). A 429, 5xx or dropped connection backs off through the job retry policy.
8. **Content kept:**
   - the text body: the `text/plain` part, or the HTML converted with `html2text`;
   - the snippet (the first 200 characters of the body for IMAP) and the selected headers (`List-Id`, `List-Unsubscribe`, `Precedence`, `Auto-Submitted`, `Reply-To`, `In-Reply-To`);
   - attachment metadata (name, MIME type, size, part id).

   ICS parts are stored whole. PDF text is extracted only on demand (bill extraction, ≤ 5 MB) and stored in `attachments` jsonb as content.

## 5. The pipeline engine (D-029, 'Rules only')
- For each decision point, the active `PipelineVersion` lists its stages. For each enabled stage, in order:
  - if the stage is unavailable, apply `on_unavailable` (`skip`, `wait` or `queue`);
  - otherwise run it. A result at or above `accept_threshold` is accepted and recorded as a `Decision`; anything lower falls through to the next stage.
- **The last enabled stage must be `rules` or `human`**, enforced on save (Domain.md invariant).
- The **'Rules only'** preset (active in M3) for each email decision point:
  1. `rules` (built-in and user rules, threshold 0.8);
  2. `human` (the question goes to the labelling view's queue and a default value applies meanwhile).

  The defaults are:

  | Decision point | Default value |
  |---|---|
  | `email.category` | `other` |
  | `email.time_sensitive` | false |
  | `email.member_link` | the owners when there's one owner, otherwise `household` |
  | `email.is_bill` | false |
  | `event.calendar_route` | a proposal in the approval inbox |
- **Multi-valued `email.member_link`:** each match yields one member and its confidence. Every member at or above the threshold is linked (D-027 notes the single-label limit of System One; rules don't have it).
- **Versioning:** an edit saves a new version, and an earlier one can be reactivated. In M3, rules can be edited (Settings → Mail rules) but the stage list can't; the Pipeline settings screen is M4.
- Every decision records its stage, version, value and confidence, so M4 can report each stage's hit rate.

## 6. Classification rules (fa-rules)
### 6.1 Rule conditions
A `mail_rule` holds a list of conditions, all of which must match (AND). Each has a `field`, an `op` and `values`:
- **Fields:** `from_addr`, `from_domain` (matches subdomains), `to`, `subject`, `body`, `gmail_label`, `header:<Name>`, `has_attachment_type`, `org_kind`, `org`, `source`.
- **Ops:** `is`, `in`, `contains_any` (whole word, case-insensitive), `matches` (regex, length-limited), `exists`.

Rules are evaluated by priority, and the first match for a decision point wins. Rules are data: they are edited in the UI and exported to TOML (D-018).

### 6.2 Built-in signals (each can be turned off)
| Signal | Decision point → value | Confidence |
|---|---|---|
| Gmail `SPAM` label (Gmail API or `X-GM-LABELS`) | category → `junk` | 0.99 |
| Gmail `CATEGORY_PROMOTIONS` / `CATEGORY_SOCIAL` | category → `promotions` / `social` | 0.85 |
| Sender is an `org_contact` | category → the org's kind | 0.95 |
| Sender's address or domain belongs to an `org` | category → the org's kind | 0.9 |
| `List-Unsubscribe` or `Precedence: bulk` with no org match | category → `newsletters` | 0.8 |
| "Payment received", "thank you for your payment", "autopay scheduled" | is_bill → false (D-059) | 0.85 |
| Biller org, or bill phrases ("amount due", "statement is ready", "payment due") **plus** a currency amount | is_bill → true | 0.9 / 0.8 |
| Contact enrolled with a child | member_link → that child | 0.95 |
| Org match and the child's name or alias in the subject or body | member_link → that child | 0.9 |
| Org match and a grade or class phrase ("Grade 3", "3rd grade", "Year 3", "Kindergarten", the class name) matching an enrollment | member_link → those children | 0.85 |
| Org match and a child's relevance tag, or a gender term for that child's `gender` (D-055) | member_link → those children | 0.8 |
| Org match with no child signal | member_link → every child enrolled there | 0.6 (falls through to human) |
| Unknown sender at a known org's domain mentioning a child (name, alias, grade, class) | contact suggestion (D-056), not a decision | n/a |
| Urgency phrases ("today", "tomorrow", "cancelled", "closed", "early dismissal", "action required") **and** a date within 48 h | time_sensitive → true | 0.85 |

### 6.3 School and daycare relevance
PROJECT.md asks for school and daycare mail to be filtered to the details that matter for each child's age, grade and gender. Children carry an optional `gender` and free-form `relevance_tags` (D-055). In M3 the deterministic part is:
- link the message to the children it concerns (§6.2);
- split a newsletter into sections (HTML headings, rules, bold lines, or blank-line blocks with a short title line);
- for each linked child, a section is **relevant** when it names the child, their grade, class, a relevance tag, or a gender term for their gender (household term lists, default `female`: girls, girls', girl's; `male`: boys, boys', boy's); it is **other** when it names only another grade or only the other gender's terms; otherwise **general**;
- the message view, filtered to a child, shows relevant and general sections and collapses the others. Gender is used only for this and is never shown on the dashboard.

Summarising a long newsletter per child is System 2 work (M4).

## 7. Extraction
| Extractor | Produces | Confidence |
|---|---|---|
| `text/calendar` parts and `.ics` attachments | events (start, end, location, UID) and cancellations (`METHOD:CANCEL`) | 0.95 |
| schema.org JSON-LD and microdata in the HTML body (`Event`, `EventReservation`, `FoodEstablishmentReservation`, `LodgingReservation`, `FlightReservation`) | events | 0.9 |
| Biller templates (per `org`: an amount regex, a due-date regex, an account-hint regex) | bills | 0.9 |
| Generic bill parser (amount near "due"/"total", a due date near "due"/"by"; PDF text when the body has no amount, D-009) | bills | 0.6–0.8 |
| Date-phrase finder ("Friday, Nov 7 at 6pm", "11/7 6:30–8pm", "tomorrow at 3") resolved against `received_at` in the household time zone, with the title taken from the subject or the sentence | events | 0.5–0.7 |
| Action phrases ("please sign", "permission slip", "RSVP by", "reply by", "return by") with an optional deadline | suggested tasks (D-060) | 0.6–0.8 |
| A direct question (`?` in a sentence addressed to the reader) from an `org_contact` | suggested reply task (adults) | 0.6 |

- Extraction runs only for categories whose behaviour is `extract` (D-054).
- Junk is never extracted (D-012: no deals).
- A pay task is created only after a bill is accepted.

## 8. Events: dedupe, routing, writes (D-008)
1. **Dedupe:** `fa-mail` inserts an `extracted` event with a `dedupe_key`: the normalised title, the start rounded to 15 minutes, and the date. `calendar.route` first looks for an existing event across every enabled calendar with:
   - the same key; or
   - a title similarity of at least 0.85 (`strsim`, Jaro-Winkler) and a start within ±60 minutes, or the same date for all-day events.

   A match links the message to the existing event (provenance) and stops there. An ICS UID match is exact.
2. **Routing**, the `event.calendar_route` pipeline (rules, then human). The rules are:
   - the org's `default_calendar`;
   - the calendar marked `is_default_target` for the linked member when there's exactly one;
   - the household default target.

   Learned routing comes in M6.
3. **Gate (D-008, D-057):** in M3 `events.auto_write` defaults to **off**, so every extracted event is **proposed** with its suggested calendar. When an adult turns it on, an event whose extraction and routing confidences are both at or above the thresholds (default 0.9 / 0.9) is written directly.
4. **Write** (on approval, or through the gate when auto-write is on):
   - **Google:** `events.insert` with scope `calendar.events`, granted by incremental consent per account.
   - **CalDAV:** `PUT` of a VEVENT with our own UID.
   - The source must be `writable`, which an adult turns on per source.

   The written event is stored with `origin = extracted`, `status = written`, its remote UID and its `source_ref`. The next sync matches it by UID, so it isn't duplicated.
5. **Cancellations:**
   - an ICS `CANCEL` or a matching "cancelled" message for an event we wrote proposes deleting it (deletes it only when auto-write is on and the match is certain);
   - a cancellation for a still-proposed event withdraws the proposal;
   - an event that came from someone else's calendar is never deleted.

## 9. Bills (D-009, D-059) and tasks from email (D-060)
- A bill at or above the threshold (0.8) becomes `open`; below it, it is `proposed` and waits in the inbox. Confirming sets it `open`.
- An `open` bill gets a "Pay <payee>" task from `tasks.from_bill`: the **pool** strategy over all adults, due `lead_days` (default 3) before `due_on`, on the household's bills list (kind `general`, created on first use).
- **Paid** means the pay task was completed; undoing the completion reopens the bill. There is no autopay flag and no payment-email matching: "payment received" mail is marked not-a-bill (§6.2).
- **Email tasks**, per category `email_tasks` (default `suggest`):
  - `suggest`: action phrases and contact questions create a task with status `suggested` on the list of kind `email`, shown only in the inbox until accepted (one tap, assignees pre-filled from the member links; reply tasks go to the adult owners);
  - `create`: the same task is created `open`;
  - `off`: nothing automatic.
  - "Make task" on any message is always available to adults who can see it.

## 10. The approval inbox and the labelling view
- **Inbox** (`/inbox` in the app, and an `inbox` panel on the dashboard) holds four kinds:
  - **events** (every extracted event in M3, D-057): approve, which can change the calendar, title or time; or reject;
  - **bills** below threshold: confirm, edit or dismiss;
  - **tasks** (suggested, D-060): accept (optionally editing assignees and due) or dismiss;
  - **contacts** (suggested, D-056): accept, which creates the contact and links it to the enrollment, or reject (remembered per address).
  - Each item shows its "from email ↗" link (in-app message view when the viewer can see the message).
- Approving needs an adult, or an elevated dashboard with a PIN (D-019).
- Every approval, edit or rejection is stored as a `Decision` from the `human` stage, which is training data for M4 kNN and M6 routing.
- **Expiry (D-061):** `inbox.expire` marks proposals `expired` (not rejected, so they aren't negatives): events when their start passes, suggested tasks and contacts after 14 days (household settings); bills never.
- **Notifications (D-061):** `approval_needed` is a per-adult trigger, default **batched** (D-044) plus the daily digest; each adult can switch it to immediate, digest only, or off.
- **Mail actions (D-058):** after processing, a writable source applies its category's `auto_actions` (default: labels `FA/<category>` and `FA/<child>`; archive and mark read are opt-in per category). Each is a `gmail_action` row, undoable from the message view for 30 days.
- **Labelling view** (`/mail/label`, D-062):
  - one message at a time, with all four email decision points (category, time-sensitive, member links, is-bill) pre-filled from the current decisions, one-tap confirm or fix;
  - "next" comes from a suggested queue stratified by category and by rule-decided vs fell-to-default; any message can also be labelled from the message view;
  - an "in eval sample" toggle and progress toward ~200 messages (D-028).

## 11. API (sketch; plan/M3.md fixes the final table)
| Method & path | Actor | Purpose |
|---|---|---|
| `GET/POST /mail-sources`, `PATCH/DELETE /mail-sources/{id}`, `POST /mail-sources/{id}/sync`, `POST /mail-sources/imap/test` | adult (owner) | Sources (Gmail or IMAP), owners, visibility, filter, read-only, sync now, IMAP connection test |
| `GET /messages?category&member&source&state&q`, `GET /messages/{id}` | adult (visible sources, D-063) | List and detail with decisions, derived items, sections |
| `POST /messages/{id}/labels` `{decision_point, value}` | adult | Fix category / member / time-sensitive / bill |
| `POST /messages/{id}/eval-sample` `{in}` | adult | Labelling view |
| `POST /messages/{id}/actions` `{archive\|mark_read\|label}`, `POST /gmail-actions/{id}/undo` | adult who can see the message, source not read-only | D-007, D-058 actions |
| `POST /messages/{id}/task` | adult | Make a task from a message |
| `GET/POST/PATCH /orgs`, `/orgs/{id}/contacts`, `/members/{id}/enrollments` | adult | Linking setup |
| `GET/POST/PATCH /mail-rules`, `POST /mail-rules/test` `{rule, message?}` | adult | Rules, and dry-running a rule on recent mail |
| `GET/POST/PATCH /categories` | adult | Category set, behaviour, auto actions, email tasks |
| `GET /inbox`, `POST /inbox/{kind}/{id}/approve\|reject` (kind: `event\|bill\|task\|contact`) | adult, elevated device | Approvals |
| `GET /bills`, `PATCH /bills/{id}` | adult | Bills list (no ledger, D-009) |

## 12. Authorization (adds to the RBAC matrix in Domain.md)
| Action | Child | Adult | Dashboard | Elevated dashboard |
|---|---|---|---|---|
| See messages | ✗ (derived events and tasks only) | sources they own, and `household` sources (D-063) | ✗ | ✗ |
| Fix labels, label for eval | ✗ | ✓ (visible messages) | ✗ | ✗ |
| Mail actions and undo (D-058) | ✗ | adults who can see the message | ✗ | ✗ |
| Manage sources | ✗ | own and household | ✗ | ✗ |
| Manage orgs, contacts, enrollments, rules, categories | ✗ | ✓ | ✗ | ✗ |
| See the inbox | ✗ | ✓ | count only | ✓ |
| Approve or reject | ✗ | ✓ | ✗ | ✓ (D-019) |

## 13. Jobs
| Job | Schedule | What it does |
|---|---|---|
| `mail.sync_due` | every minute | Enqueues `mail.sync{source}` for due sources |
| `mail.sync{source}` | enqueued | First or incremental sync (§4) |
| `mail.process{message}` | enqueued | Pipelines, extractors, emit (§5 to §9) |
| `calendar.route{event}` | enqueued | Dedupe, route, gate, write or propose (§8) |
| `tasks.from_bill{bill}`, `tasks.from_mail{message, kind}` | enqueued | Pay and email tasks (§9) |
| `inbox.expire` | hourly | Marks stale proposals `expired` (D-061) |
| `mail.retention` | daily | The D-034 policy: purge content, mark expired after `max_pending_days` |

## 14. Configuration (D-018)
The household TOML gains:
- `[[orgs]]`, with `contacts` and the ICS feed name;
- `[[members]].enrollments` and `aliases`;
- `[[mail_sources]]`, with its provider, address, owners, visibility, read-only, backfill, query, folders (IMAP) and poll interval. Credentials (OAuth or the app password) are entered in the UI, so the import leaves the source in `needs_reauth`;
- `[[members]].gender`, `relevance_tags`, and `[household.gender_terms]`;
- `[household.events]` (`auto_write`, thresholds) and `[household.inbox]` (expiry);
- `[[mail_rules]]`;
- `[[categories]]` (behaviour, notify, auto actions, email tasks);
- `[retention.mail]`;
- the thresholds.

Secrets are never exported.

## 15. Frontend (summary; detail in Frontend.md v3)
New app screens:
- **Mail:** the list by category and member, and the message detail;
- **Inbox**;
- **Labelling**;
- **Bills**;
- Settings → Mail (sources with the Gmail/IMAP choice, categories, rules);
- Settings → Organizations (orgs, contact mode, contacts, enrollments);
- the member editor gains aliases, gender and relevance tags (children).

New dashboard panel: `inbox` (a count, and items when elevated).

## 16. Security and privacy notes
- Email bodies are not secrets, so they're stored in plain text under the retention policy (D-034). OAuth tokens and IMAP app passwords are `Secret`s and are never exported.
- Message content never appears in logs. Log lines carry ids, counts and timings only (D-035 and the no-real-data rule).
- `matches` regexes are length-limited and compiled with a size limit, so a bad user rule can't stall the worker.
- PDF parsing runs with a size cap and a timeout in a blocking task. A failure leaves the bill `proposed`.
- IMAP connections require TLS (port 993, or STARTTLS on 143); plain-text login is refused.
- Calendar writes and mail actions are audited (D-013 audit log) and limited to sources an adult has made writable.
