---
title: Ingest — email, rules pipeline, bills, linking, approvals
scope: Gmail sources and sync, message storage and retention, the decision pipeline engine (rules-only preset), deterministic classifiers and extractors, orgs, contacts and enrollments, bills, event proposals and calendar writes, the approval inbox, email tasks, the labelling view
status: draft
updated: 2026-10-25
drawio: Ingest.drawio
drawio_src: src/Ingest.py
depends_on: [D-001, D-004, D-006, D-007, D-008, D-009, D-010, D-012, D-013, D-014, D-018, D-019, D-023, D-024, D-027, D-028, D-029, D-034, D-035, D-036, D-044, D-045]
milestone: M3
---

# Ingest: email, rules pipeline, bills, linking, approvals (DRAFT v0)

This doc covers M3 (D-023, as adjusted by D-029):
- Gmail ingest;
- the decision pipeline engine with the **'Rules only'** preset;
- deterministic classification and extraction;
- org, teacher and child linking;
- bills (D-009);
- event proposals with calendar writes and the approval inbox (D-008);
- tasks from email;
- the labelling view that builds the eval sample (D-028).

v0 is derived from PROJECT.md and the accepted decisions. **Batch 16 (Q-049 to Q-060) is open.** The items marked *(Q-NNN)* change when it is answered, and v1 re-derives this doc from the answers.

**Not in M3:**
- System One, kNN, the LLM (System 2) and their stages; Pipeline and Backends settings screens; `fa-eval` (all M4, `AI.md`);
- learned calendar routing (M6);
- deals (D-012), school portal APIs (D-014), budgeting (D-009).

## 1. Concepts
| Concept | What it is | Decided by |
|---|---|---|
| **Mail source** | A Gmail account linked to one or more owner members, read-only by default | D-007 |
| **Message** | One email. Its metadata is always kept; its content follows the retention policy | D-034 |
| **Org** | A school, daycare, club, biller or other sender group, with its domains, sender addresses and optional ICS feeds | D-014 |
| **Contact / Enrollment** | A person at an org (a teacher, a coach), and which child is enrolled where (grade, class). This is how a teacher's email reaches the right child *(Q-053)* | Domain.md |
| **Decision point** | A question asked about a subject: `email.category`, `email.time_sensitive`, `email.member_link`, `email.is_bill`, `event.calendar_route`, `task.urgency` (M4) | D-027 |
| **Pipeline** | A versioned, ordered list of stages for one decision point. M3 runs `rules` and `human` stages only | D-029 |
| **Extractor** | A deterministic producer of candidate events, bills and tasks from a message (ICS parts, schema.org data, biller templates, date phrases) | D-001, D-009 |
| **Proposal** | An extracted event, bill or task that is below the confidence threshold or has no certain target, waiting in the approval inbox | D-008 |
| **Labelling view** | Where adults confirm or correct the decisions on a message and build the ~200-message eval sample | D-028 |

## 2. Crates (page "Components")
| Crate | Responsibility | Depends on |
|---|---|---|
| `fa-connectors` (extended) | `gmail` client: `users.getProfile`, `messages.list` (with `q`), `history.list`, `messages.get` (`format=raw`), `attachments.get`, `messages.modify`, `labels.list/create`. `gcal` writes (`events.insert/patch/delete`) and CalDAV `PUT`/`DELETE` of a VEVENT. No DB access | (external crates only) |
| `fa-pipeline` (new) | The engine (§5): loads a decision point's active `PipelineVersion`, runs the enabled stages in order with thresholds and `on_unavailable`, and records `Decision` rows. The `Stage` trait has the `rules` and `human` implementations in M3; `knn`, `system_one` and `llm` are M4 and are treated as unavailable. Built-in presets, with 'Rules only' active | fa-domain, fa-store |
| `fa-rules` (new) | Deterministic rulesets and extractors, as pure functions over a parsed message (§6, §7): header and Gmail-label signals, org and contact matching, member name and grade matching, bill detection and biller templates, the date-phrase finder, ICS and schema.org extraction, PDF text | fa-domain (+ `mail-parser`, `html2text`, `scraper`, `pdf-extract`, `regex`, `aho-corasick`) |
| `fa-mail` (new) | Sources and sync (§4), message storage, processing (`mail.process`: runs the email pipelines, then the extractors, then emits proposals through the store and job queue), Gmail actions with undo, the retention job, the labelling view's service | fa-domain, fa-store, fa-jobs, fa-connectors, fa-pipeline, fa-rules |
| `fa-calendar` (extended) | `calendar.route`: dedupe, the `event.calendar_route` pipeline, the confidence gate, then either writes the event to Google or CalDAV or leaves it `proposed` for the inbox. Approve, edit and reject | + fa-pipeline |
| `fa-tasks` (extended) | `tasks.from_mail` (a reply task or a rule-made task) and `tasks.from_bill` (a "Pay <payee>" task). A paid bill closes its task, and a completed pay task marks the bill paid | (unchanged) |
| `fa-notify` (extended) | New trigger kinds: `mail_time_sensitive` and `approval_needed` | (unchanged) |
| `fa-api` (extended) | Routes (§11), SSE kinds `mail.updated` and `inbox.updated` | + fa-mail |

The coupling rule from M1 and M2 holds: **feature crates never depend on each other.** `fa-mail` emits through the store and the job queue:
- it inserts an `event` row with `status = 'extracted'`, then enqueues `calendar.route{event}`;
- it inserts a `bill` row, then enqueues `tasks.from_bill{bill}`;
- it enqueues `tasks.from_mail{message, kind}`;
- it inserts `notification` outbox rows.

`fa-pipeline` and `fa-rules` are shared infrastructure, like `fa-jobs`: any feature crate may use them.

**External crates** (checked on crates.io on 2026-10-25): `mail-parser 1.0` (MIME), `html2text 0.17`, `scraper 0.29` (schema.org JSON-LD and microdata), `pdf-extract 0.12` (D-009), `regex 1.13`, `aho-corasick 1.1` (many keyword and name matches), `strsim 0.11` (dedupe title similarity), and `icalendar 0.17` (already used). Date phrases are found by our own regex patterns and resolved with `chrono` against the message's received time and the household time zone. The natural-language date crates found (`chrono-english`, `interim`, `two_timer`) parse a whole string and cannot find dates inside free text.

## 3. Data model (page "Model")
This refines the Organizations & Mail and AI & Pipelines packages of [Domain.md](Domain.md). All tables are household-scoped with RLS.

| Table | Key columns | Notes |
|---|---|---|
| `org` | kind: `school\|daycare\|club\|biller\|other`, name, domains[], senders[], calendar_source? (ICS feed, D-014), default_calendar?, archived_at?, version | `domains` match the sender's domain or subdomains. `senders` are exact addresses |
| `org_contact` | org, name, email, role: `teacher\|coach\|admin\|office\|other`, archived_at? | Unique `(org, lower(email))` |
| `enrollment` | member, org, grade?, class?, from?, until? | A child at a school, daycare or club |
| `enrollment_contact` | enrollment ↔ org_contact | This child's teacher or coach |
| `member` (+cols) | `aliases[]` (nicknames to match in mail), `relevance_tags[]` *(Q-052)* | Used by member linking (§6.3) |
| `mail_source` | provider: `gmail`, oauth_account, address, read_only (true), backfill_days (30), query? (Gmail search filter, *Q-050*), poll_minutes (5), history_id?, state: `ok\|needs_reauth\|error\|paused`, last_synced_at?, last_error?, archived_at?, version | D-007. Reuses `oauth_account` (Feeds.md §2) with incremental consent for `gmail.readonly`, or `gmail.modify` when read-only is off |
| `mail_source_owner` | mail_source ↔ member (1..*) | The owners are the classification context and the visibility boundary *(Q-060)* |
| `message` | source, gmail_id, thread_id, rfc822_id?, received_at, from_addr, from_name?, to[], cc[], subject, gmail_labels[], has_attachments, category?, time_sensitive?, is_bill?, processing_state: `pending\|processed\|expired`, **content** (body_text?, snippet?, headers jsonb?, attachments jsonb?), is_learning_example, content_purged_at?, in_eval_sample | Unique `(source, gmail_id)`. Mail sent to several of the household's sources is one message per source; the derived items are deduped (§8) |
| `message_member` | message ↔ member, decided_by: `rule\|user` | Result of `email.member_link`, which can give several members |
| `decision_point`, `pipeline_version`, `stage`, `decision` | As in Domain.md (AI & Pipelines) | Created in M3, so M4 only adds stage kinds |
| `mail_rule` | decision_point, name, priority, conditions jsonb (§6.1), value, confidence, enabled, origin: `builtin\|user`, version | User rules for the `rules` stage. Built-in rules are code with listed defaults, and can be disabled |
| `category` | key, label, behaviour: `ignore\|keep\|extract`, notify_time_sensitive, color, ord, archived_at? | The household's category set *(Q-051)* |
| `bill` | message?, org?, payee, amount_minor, currency, due_on?, account_hint?, statement_period?, status: `proposed\|open\|paid\|dismissed`, confidence, autopay *(Q-056)*, task?, source_ref, version | D-009. `proposed` waits in the inbox |
| `event` (+use) | origin `extracted`, status `extracted\|proposed\|approved\|written\|rejected`, confidence, source_ref, dedupe_key, target_calendar? | Columns from M1 (Feeds.md §2), now used |
| `task` (+use) | origin `email`, source_ref, list kind `email` | Domain.md Tasks package |
| `gmail_action` | message, action: `archive\|mark_read\|label`, label?, done_by, done_at, undone_at? | D-007 actions are audited and undoable |

`SourceRef{kind: email, message_id, external_url}` links every derived item back to its message. `external_url` is the Gmail deep link `https://mail.google.com/mail/u/?authuser=<address>#all/<gmail_id>`.

## 4. Gmail sync
1. **Connect:** Settings → Mail → "Connect Gmail". This reuses the Google OAuth flow from M1, with incremental consent for `https://www.googleapis.com/auth/gmail.readonly`, or `gmail.modify` when read-only is off. Both are **restricted scopes**, so the household runs its own OAuth client in Testing mode, and refresh tokens expire after 7 days (D-007, *Q-049*).
2. **Owners:** the adult picks the owner members (D-007), the backfill window, and an optional Gmail search filter *(Q-050)*.
3. **Scheduling:** `mail.sync_due` runs every minute and enqueues `mail.sync{source}` when `last_synced_at + poll_minutes` has passed. The idempotency key is `mail.sync:{source}:{slot}`. Gmail push (Pub/Sub) needs a public endpoint, so it is not used on a tailnet-only deployment.
4. **First sync:**
   - read the `historyId` from `users.getProfile`;
   - page through `messages.list?q=newer_than:{backfill_days}d {query}`;
   - fetch each new message with `messages.get?format=raw`, with up to 4 in flight;
   - parse it with `mail-parser`;
   - store it `pending` and enqueue `mail.process{message}`.
5. **Incremental sync:**
   - call `history.list?startHistoryId=…&historyTypes=messageAdded&historyTypes=labelAdded&historyTypes=labelRemoved`;
   - new messages are fetched as in step 4;
   - label changes update `gmail_labels`. Moving a message to SPAM or Trash re-runs only `email.category`.
   - A **404** means the history ID is too old (Google keeps it at least a week, sometimes only hours). The source then falls back to a full sync of the backfill window, which is safe because `(source, gmail_id)` is unique.
6. **Errors:** `invalid_grant` sets the `oauth_account` to `needs_reauth`. That pauses every source using it, and a `source_reauth` notification goes out (M2 trigger). A 429 or 5xx backs off through the job retry policy.
7. **Content kept:**
   - the text body: the `text/plain` part, or the HTML converted with `html2text`;
   - the snippet and the selected headers (`List-Id`, `List-Unsubscribe`, `Precedence`, `Auto-Submitted`, `Reply-To`, `In-Reply-To`);
   - attachment metadata (name, MIME type, size, attachment id).

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
| Gmail `SPAM` label | category → `junk` | 0.99 |
| Gmail `CATEGORY_PROMOTIONS` / `CATEGORY_SOCIAL` | category → `promotions` / `social` | 0.85 |
| Sender is an `org_contact` | category → the org's kind | 0.95 |
| Sender's address or domain belongs to an `org` | category → the org's kind | 0.9 |
| `List-Unsubscribe` or `Precedence: bulk` with no org match | category → `newsletters` | 0.8 |
| Biller org, or bill phrases ("amount due", "statement is ready", "payment due") **plus** a currency amount | is_bill → true | 0.9 / 0.8 |
| Contact enrolled with a child | member_link → that child | 0.95 |
| Org match and the child's name or alias in the subject or body | member_link → that child | 0.9 |
| Org match and a grade or class phrase ("Grade 3", "3rd grade", "Year 3", "Kindergarten", the class name) matching an enrollment | member_link → those children | 0.85 |
| Org match with no child signal | member_link → every child enrolled there | 0.6 (falls through to human) |
| Urgency phrases ("today", "tomorrow", "cancelled", "closed", "early dismissal", "action required") **and** a date within 48 h | time_sensitive → true | 0.85 |

### 6.3 School and daycare relevance
PROJECT.md asks for school and daycare mail to be filtered to the details that matter for each child's age, grade and gender. In M3 the deterministic part is:
- link the message to the children it concerns (§6.2);
- in a multi-section newsletter, keep a section only when it names one of the linked children's grade, class, or **relevance tags** *(Q-052)*;
- the message view shows the kept sections first and the rest collapsed.

Summarising a long newsletter per child is System 2 work (M4).

## 7. Extraction
| Extractor | Produces | Confidence |
|---|---|---|
| `text/calendar` parts and `.ics` attachments | events (start, end, location, UID) and cancellations (`METHOD:CANCEL`) | 0.95 |
| schema.org JSON-LD and microdata in the HTML body (`Event`, `EventReservation`, `FoodEstablishmentReservation`, `LodgingReservation`, `FlightReservation`) | events | 0.9 |
| Biller templates (per `org`: an amount regex, a due-date regex, an account-hint regex) | bills | 0.9 |
| Generic bill parser (amount near "due"/"total", a due date near "due"/"by"; PDF text when the body has no amount, D-009) | bills | 0.6–0.8 |
| Date-phrase finder ("Friday, Nov 7 at 6pm", "11/7 6:30–8pm", "tomorrow at 3") resolved against `received_at` in the household time zone, with the title taken from the subject or the sentence | events | 0.5–0.7 |
| Action phrases ("please sign", "permission slip", "RSVP by", "reply by", "return by") with an optional deadline | tasks *(Q-057)* | 0.6–0.8 |

- Extraction runs only for categories whose behaviour is `extract` (*Q-051*).
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
3. **Gate:** if the extraction confidence and the routing confidence are both at or above the thresholds (household defaults, which can be set per category, D-008, *Q-054*), the event is **written**. Otherwise it is **proposed**.
4. **Write:**
   - **Google:** `events.insert` with scope `calendar.events`, granted by incremental consent per account.
   - **CalDAV:** `PUT` of a VEVENT with our own UID.
   - The source must be `writable`, which an adult turns on per source.

   The written event is stored with `origin = extracted`, `status = written`, its remote UID and its `source_ref`. The next sync matches it by UID, so it isn't duplicated.
5. **Cancellations:**
   - an ICS `CANCEL` or a matching "cancelled" message for an event we wrote proposes deleting it, or deletes it when confident;
   - an event that came from someone else's calendar is never deleted.

## 9. Bills (D-009) and tasks from email
- A bill at or above the threshold becomes `open`, and `tasks.from_bill` creates "Pay <payee>" due `lead_days` before `due_on`. The assignee comes from the payee org, or else the source owners *(Q-056)*. A bill below the threshold is `proposed`.
- Completing the pay task marks the bill `paid`, and undoing it reopens the bill. A payment confirmation from the same biller that matches the amount marks the bill paid and closes its task *(Q-056)*.
- Tasks from email go to a list of kind `email`. Each has a "Reply" or action title, the deadline as `due_at`, assignees from the member links (adults only for reply tasks), and a `source_ref`. Any adult can also turn a message into a task by hand *(Q-057)*.

## 10. The approval inbox and the labelling view
- **Inbox** (`/inbox` in the app, and an `inbox` panel on the dashboard):
  - **events:** approve, which can change the calendar, title or time; or reject;
  - **bills:** confirm, edit or dismiss;
  - **tasks:** accept or dismiss;
  - each item shows its "from email ↗" link.
- Approving needs an adult, or an elevated dashboard with a PIN (D-019).
- Every correction is stored as a `Decision` from the `human` stage, which is training data for M4 kNN and M6 routing.
- Expired proposals (event date passed, bill paid) leave the inbox automatically *(Q-058)*.
- A new proposal notifies adults with `approval_needed` (batched like other notifications, D-044).
- **Labelling view** (`/mail/label`):
  - one message at a time, showing the current value of each email decision point with one-tap confirm or fix;
  - an "add to eval sample" toggle, and progress toward ~200 messages (D-028);
  - the system suggests messages stratified by category and by rule-versus-default, so the sample isn't all newsletters *(Q-059)*;
  - it shares the "Fix category" action with the message view.

## 11. API (sketch; plan/M3.md fixes the final table)
| Method & path | Actor | Purpose |
|---|---|---|
| `GET/POST /mail-sources`, `PATCH/DELETE /mail-sources/{id}`, `POST /mail-sources/{id}/sync` | adult (owner) | Sources, owners, filter, read-only, sync now |
| `GET /messages?category&member&source&state&q`, `GET /messages/{id}` | adult (visible sources, *Q-060*) | List and detail with decisions, derived items, sections |
| `POST /messages/{id}/labels` `{decision_point, value}` | adult | Fix category / member / time-sensitive / bill |
| `POST /messages/{id}/eval-sample` `{in}` | adult | Labelling view |
| `POST /messages/{id}/actions` `{archive\|mark_read\|label}`, `POST /gmail-actions/{id}/undo` | adult (owner, source not read-only) | D-007 actions |
| `POST /messages/{id}/task` | adult | Make a task from a message |
| `GET/POST/PATCH /orgs`, `/orgs/{id}/contacts`, `/members/{id}/enrollments` | adult | Linking setup |
| `GET/POST/PATCH /mail-rules`, `POST /mail-rules/test` `{rule, message?}` | adult | Rules, and dry-running a rule on recent mail |
| `GET/PATCH /categories` | adult | Category set and behaviour |
| `GET /inbox`, `POST /inbox/{kind}/{id}/approve\|reject` | adult, elevated device | Approvals |
| `GET /bills`, `PATCH /bills/{id}` | adult | Bills list (no ledger, D-009) |

## 12. Authorization (adds to the RBAC matrix in Domain.md)
| Action | Child | Adult | Dashboard | Elevated dashboard |
|---|---|---|---|---|
| See messages | ✗ *(Q-060)* | sources they own *(Q-060)* | ✗ | ✗ |
| Fix labels, label for eval | ✗ | ✓ (visible messages) | ✗ | ✗ |
| Gmail actions *(Q-055)* | ✗ | owners of the source | ✗ | ✗ |
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
| `inbox.expire` | hourly | Drops stale proposals |
| `mail.retention` | daily | The D-034 policy: purge content, mark expired after `max_pending_days` |

## 14. Configuration (D-018)
The household TOML gains:
- `[[orgs]]`, with `contacts` and the ICS feed name;
- `[[members]].enrollments` and `aliases`;
- `[[mail_sources]]`, with its address, owners, read-only, backfill, query and poll interval. Connecting a source still needs OAuth in the UI, so the import leaves it in `needs_reauth`;
- `[[mail_rules]]`;
- `[[categories]]`;
- `[retention.mail]`;
- the thresholds.

Secrets are never exported.

## 15. Frontend (summary; Frontend.md v3 after batch 16)
New app screens:
- **Mail:** the list by category and member, and the message detail;
- **Inbox**;
- **Labelling**;
- **Bills**;
- Settings → Mail (sources, categories, rules);
- Settings → Organizations (orgs, contacts, enrollments).

New dashboard panel: `inbox` (a count, and items when elevated).

## 16. Security and privacy notes
- Email bodies are not secrets, so they're stored in plain text under the retention policy (D-034). OAuth tokens are `Secret`s.
- Message content never appears in logs. Log lines carry ids, counts and timings only (D-035 and the no-real-data rule).
- `matches` regexes are length-limited and compiled with a size limit, so a bad user rule can't stall the worker.
- PDF parsing runs with a size cap and a timeout in a blocking task. A failure leaves the bill `proposed`.
- Calendar writes and Gmail actions are audited (D-013 audit log) and limited to sources an adult has made writable.
