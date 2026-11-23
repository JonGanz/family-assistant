# Improvements backlog

Open items only. Closed items were removed on 2026-11-23; their history is in
[plan/reviews/2026-10-26-repo-review.md](plan/reviews/2026-10-26-repo-review.md),
[plan/reviews/2026-11-02-code-review-2.md](plan/reviews/2026-11-02-code-review-2.md), [plan/M3.5.md](plan/M3.5.md) and
[plan/M3.7.md](plan/M3.7.md). Size: **QUICK** = a sub-agent brief under 10 minutes, **LARGER** = a milestone task or a
question first.

## Needs your decision (repository review, 2026-11-23)
Findings from the 2026-11-23 review that were **not** fixed on the `review-fixes-2026-11-23` branch because the fix
changes behaviour, security posture or process. Each needs an answer (a `Q-NNN` where it is a design question).

### Release and process
| ID | Finding | Decision needed | Size |
|---|---|---|---|
| V-01 | `ci.yml` runs only by hand, so a `v*` tag publishes an image with no gate | Turn CI on for pull requests and tags, and make the release job depend on it? (It was switched off on purpose.) | QUICK |
| V-02 | Tags are lightweight and unsigned; no GitHub Release body | Annotated or signed tags; release notes from the changelog entry? | QUICK |
| V-03 | `deploy/compose.yaml` defaults `FA_VERSION` to `latest`; `ollama/ollama:latest` is unpinned | Pin the published compose file to a release per tag, or keep `latest`? Which Ollama tag? | QUICK |
| V-04 | `rust-toolchain.toml` is `stable` and the Dockerfile uses `rust:1-bookworm`; `rust-version = 1.88` is never tested | Pin an exact toolchain (and bump it by Dependabot), add an MSRV job? | QUICK |
| V-05 | Only a container image and a stub GPU agent are released | `fa-server` tarballs (amd64, arm64), a `.deb` with a systemd unit, image signing (cosign)? | LARGER |
| V-06 | The version is bumped by hand in three places (workspace, web, changelog) | Adopt `cargo-release` or `release-plz`? | QUICK |
| V-07 | No upgrade test from the previous release's schema; no documented rollback or "minimum upgradable from" version | Add both to the release checklist? | LARGER |
| V-08 | `fa-gpu-agent` is a stub that is built, released and given a systemd unit | Keep releasing it, or drop it from the release until M4.1? | QUICK |
| V-09 | No `CONTRIBUTING.md`, `SECURITY.md`, issue or PR templates; no in-app link to the source (AGPL, network use) | Add them; where does the source link go (About page)? | QUICK |
| V-10 | `README.md` is 66 KB; `STATUS.md` is stale (2026-11-08); `plan/_index.md` shows M3.7 "running" and M5 "planned"; code review 2 status column is stale | Split the README into `docs/`? Regenerate or drop `STATUS.md`? | QUICK |

### Security
| ID | Finding | Decision needed | Size |
|---|---|---|---|
| S-01 | The session cookie value is the `session.id` primary key in plaintext (`fa-auth/src/login.rs:172`); DB or backup read gives live sessions | Store a hash of the token (logs every session out on upgrade)? | LARGER |
| S-02 | The master key download does not require a secure connection (`fa-api/src/system.rs:63`) | Refuse over plain HTTP, or warn (D-072 keeps the app on HTTP)? | QUICK |
| S-03 | PIN lockout says it "notifies the adults" but `AuthEvents` is never wired | Which notification, to whom, how often? | QUICK |
| S-04 | AI endpoint test with a kept key and a new `base_url` sends the stored key there (`fa-api/src/ai.rs`); a CalDAV patch does the same with the password (`fa-calendar/src/service.rs:506`) | Require the secret again when the host changes (as C2-07 did for IMAP)? | QUICK |
| S-05 | LLM prompt injection: mail body undelimited, confidence self-reported, auto-actions follow with no human step (`fa-mail/src/process/llm.rs:337`) | Cap LLM-driven auto-actions, or require approval for LLM-set categories? | LARGER |
| S-06 | No Content-Security-Policy for the app shell | Which policy (the kiosk and Swagger UI constrain it)? | LARGER |
| S-07 | SSE checks auth only at connect; a revoked device keeps receiving change events | Re-check on an interval, or close streams on revoke? | QUICK |
| S-08 | Rate-limit buckets are per exact IP, in memory, and reset on every settings restart | Persist them; bucket IPv6 by /64? | LARGER |
| S-09 | `job` and `schedule` have no row-level security | Matters only with more than one household: add RLS now or leave? | LARGER |
| S-10 | argon2 still runs inside an open transaction on account create, reset and PIN set (`fa-api/src/routes.rs:643`, `:749`, `fa-auth/src/accounts.rs:52`) | Same fix as C-12; needs a transaction restructure | QUICK |
| S-11 | The ntfy topic is stored in plaintext; master key and DEKs are not zeroized | Encrypt the topic (C2-30 covers rotation); add `zeroize`? | QUICK |

### Behaviour
| ID | Finding | Decision needed | Size |
|---|---|---|---|
| B-01 | Saving a System setting in `serve all` waits for in-flight jobs; the API is down for up to 4 minutes (`bins/fa-server/src/serve.rs:392`) | Restart only the API, or cancel jobs with a deadline? | LARGER |
| B-02 | One 30 s request timeout covers inline LLM calls of 120 to 600 s (`fa-api/src/lib.rs:507`) | Exempt the AI routes, or move extraction to a job? | LARGER |
| B-03 | Rolling session expiry upserts the session, so a request racing logout re-creates it (`fa-auth/src/resolve.rs:63`) | Needs an `UPDATE … WHERE id` in `fa-store` (a new checked query) | QUICK |
| B-04 | The morning digest has no latest hour; an empty morning sends "Good morning" whenever the first task appears (`fa-tasks/src/tick.rs:181`) | Latest hour: configurable, default noon? | QUICK |
| B-05 | Overdue and due-soon dedupe keys are permanent; a task that goes overdue again never notifies (`fa-tasks/src/tick.rs:50`) | Notify again after a reschedule or reopen? | QUICK |
| B-06 | Events the app wrote and the user deleted at the provider stay in the family calendar (`fa-store/src/repo/event.rs:137`) | Delete them, or mark them "removed at provider"? | LARGER |
| B-07 | Google event insert has no client-chosen id; a retry after a lost response duplicates the event (`fa-calendar/src/ingest.rs:633`) | Deterministic event id, as CalDAV has | QUICK |
| B-08 | One discover response without a calendar archives it, disables it and wipes its events (`fa-calendar/src/service.rs:760`) | Require N consecutive misses? | QUICK |
| B-09 | DTSTART is always an occurrence, even when it does not match the RRULE (`fa-calendar/src/expand.rs:420`) | Follow Google (drop it) or RFC 5545 (keep it)? | QUICK |
| B-10 | A cancellation mail misses an event that is approved with its write still queued (`fa-calendar/src/ingest.rs:203`) | Cancel the queued write? | QUICK |
| B-11 | Notification delivery holds one transaction across every ntfy publish; a late failure re-sends earlier pushes (`fa-notify/src/send.rs:99`) | Commit per group | LARGER |
| B-12 | A `tonight` weather rule that first matches after midnight creates a task due yesterday (`fa-weather/src/eval.rs:172`) | Due date for early-morning matches? | QUICK |
| B-13 | Grocery: a unit-only patch shows "480 cups"; undo of an older generate under-counts (`fa-meals/src/grocery.rs:433`, `:898`) | Convert on unit change; make undo delta-based? | LARGER |
| B-14 | Migration `20261029000001_m36_member_hue` set every member colour to blue; deployed, so it cannot be edited | Accept, or restore from backups by hand? | — |
| B-15 | Schedules are upserted with `enabled = true` and the code's cron at every start; an operator's change in the DB is overwritten | Are schedules meant to be operator-editable? | QUICK |
| B-16 | `create_household` commits before the base URL is saved; a failure leaves setup closed without a URL (`fa-api/src/setup.rs:97`) | One transaction (touches `fa-config`) | QUICK |
| B-17 | Payee from `billing@x.co.uk` is "Co"; header rules on headers that are not stored never match; non-Latin member names share one IMAP keyword | Public-suffix list? Reject unknown headers or store more? Keyword encoding? | QUICK each |
| B-18 | Recipe pages are always decoded as UTF-8; EXIF orientation is ignored on recipe photos; "1-inch piece ginger" parses as quantity 1 | Add `encoding_rs`; rotate on intake; ingredient grammar | QUICK each |
| B-19 | `AuthConfig`, `WorkerConfig` and the pool size are hard-coded, against the "configurable" rule | Which become settings? | LARGER |

### Linux and operations
| ID | Finding | Decision needed | Size |
|---|---|---|---|
| L-01 | Master key default paths disagree (`/run/secrets/fa_master_key` vs `/run/fa-secrets`); `/run` is tmpfs on a bare host | One default; `/var/lib/family-assistant` outside Docker? | QUICK |
| L-02 | `init`: a failed write leaves a truncated key that is later accepted; directory not 0700; uid 10001 hard-coded; no mode check on load | Write to a temp file and rename; refuse a key readable by others? | QUICK |
| L-03 | A wrong `FA_CONFIG` path is ignored; `fa.toml` and `.env.dev` are CWD-relative and `.env.dev` loads in release builds; typoed `FA_*` variables are ignored | Fail on a missing explicit path; `/etc/family-assistant/fa.toml`; warn on unknown variables? | QUICK |
| L-04 | A second SIGTERM does nothing; jobs get no cancel token | Second signal exits at once? | QUICK |
| L-05 | The image `HEALTHCHECK` hits `/healthz`, which fails under `serve worker`; the app and ntfy publish on `0.0.0.0` by default | Worker health file; default bind `127.0.0.1`? | QUICK |
| L-06 | No shell completions or man page | `clap_complete` and `clap_mangen` subcommands? | QUICK |

### Code health
| ID | Finding | Decision needed | Size |
|---|---|---|---|
| H-A1 | Workspace lints are only `unsafe_code`, `dbg_macro`, `todo` | Add `unwrap_used`, `indexing_slicing`, `string_slice`, `arithmetic_side_effects` (many existing hits to fix or allow) | LARGER |
| H-A2 | 358 runtime `sqlx::query` calls against 207 checked macros | Convert M3 to M5 SQL to checked macros | LARGER |
| H-A3 | Files named after task briefs (`fa-store/src/repo/ext_t05.rs` and 12 more; tests `m1.rs` to `m5.rs`) | Rename by domain | LARGER |
| H-A4 | `fa-domain` depends on `sqlx`, `utoipa`, `ts-rs`; `fa-config` depends on `fa-store` and writes to the DB | Feature-gate or invert | LARGER |
| H-A5 | Oversized files: `fa-api/src/feeds.rs` (1,911 lines), `mail.rs`, `fa-tasks/src/service.rs`, `fa-domain/src/authz.rs` | Split | LARGER |
| H-A6 | Likely unused dependencies in about ten crates (grep only) | Add `cargo-machete` to CI and remove them | QUICK |
| H-A7 | Sentinel strings as errors (`LlmError::Config("disabled")`, `LlmError::Schema("no recipe")`); stringly fields (`Job.status`, `Household.tz` with a silent UTC fallback) | Typed variants; validate the time zone at write | LARGER |
| H-A8 | Resource limits: HTML walkers recurse without a depth cap (`fa-rules/src/sections.rs:102`, `fa-recipe`); mail parsing and PDF extraction run on the async worker inside a transaction; IMAP fetch has no size check (C2-05); 20 s timeout against 40 MB Gmail bodies | Depth caps, `spawn_blocking`, size checks | LARGER |
| H-A9 | `upsert_synced` does one INSERT per occurrence (up to 20,000 per sync); the `job` claim's reclaim branch has no index; `notification` is never pruned | Batch insert; a new migration for the index; notification retention period | LARGER |

### Feature gaps
| ID | Gap | Note |
|---|---|---|
| F-01 | TUI (stretch goal in `PROJECT.md`) | Not started |
| F-02 | "Spam can be filtered for possible deals of interest" (`PROJECT.md`) | Not found in the review; confirm |
| F-03 | GPU agent and System One sidecars | Placeholders (M4.1) |
| F-04 | Failed-job view with retry; alerts for sync failures; a metrics endpoint; an update-available check | Operations |
| F-05 | Outbound ICS feed of the family calendar; full data export and import; offline shopping list; bill due reminders | Confirm what exists before planning |

## Shakedown (still open)
- **R-01** Nothing has run on real data: human steps H-02 to H-11 are open.
- **R-02** The Echo Show is unproven on Silk over plain HTTP (keep-alive, SSE, service worker).
- **R-03** Measure proposals per day and the approve/reject ratio before sizing more AI work.
- **R-04** H-11 asks for a week of manual labelling; consider shrinking it.
- **U-16** The nav, page shell and kit were verified by lint, tests and a build, not in a browser or on the Echo Show.

## Code review 2: LARGER items (open)
C2-02, C2-05, C2-08, C2-09, C2-15, C2-19, C2-21, C2-30, with D-098 to D-102 designed but not built
(see [plan/reviews/2026-11-02-code-review-2.md](plan/reviews/2026-11-02-code-review-2.md)). Milestone M4.2 (D-119).

## M4 known gaps (open)
Source: the *Outcome* of [plan/M4.md](plan/M4.md). Milestone M4.2 (D-119).
| ID | Area | Gap | Suggested fix | Size |
|---|---|---|---|---|
| G4-01 | AI, notify | When the LLM sets `email.time_sensitive` to true after `mail.process` has run, no notification is created | The `mail.llm` job raises the same notification when the decision flips to true and none was sent | QUICK |
| G4-02 | AI, tasks | A task the LLM proposes before `tasks.from_mail` has run becomes a second suggestion | Order the jobs, or dedupe in `tasks.from_mail` against LLM suggestions | QUICK |
| G4-03 | AI, mail actions | When the LLM changes a category, the rules' category auto-actions stay applied; the new category's run as well | A question first: revert the old actions or keep them | LARGER |
| G4-04 | AI | A later `mail.process` rerun can hide the LLM's decision behind a new rules decision | Keep the LLM decision as authoritative in `llm_first`; re-queue `mail.llm` on rerun | QUICK |
| G4-05 | AI | Task proposals carry no confidence | Add `confidence` to the task suggestion and `TaskView` | QUICK |
| G4-06 | AI | Contacts the LLM finds carry no provenance | Add `extracted_by` to contacts and the chip in the inbox | QUICK |
| G4-07 | AI | No live test against a real OpenAI-compatible server; default model not measured | Opt-in test (`FA_TEST_LLM_URL`); needs the user's go-ahead to use the GPU | QUICK |
| G4-08 | Mail UI, API | Mail filters with several values filter only the loaded page | Multi-value query parameters on `GET /messages` | QUICK |
| G4-09 | Members | A reload between the two requests of `MembersManager` leaves a child without a login | One endpoint that creates both | QUICK |
| G4-10 | Web tests | No mount test for `CalendarView` | Add a view test | QUICK |
| G4-11 | Web | Task completion with Undo is implemented three times | A shared task-undo composable | QUICK |
| G4-12 | ui/ | Raw range sliders; `UiChip` vs `UiFilterChips` semantics; 12 `vue/require-default-prop` warnings | `UiSlider`; align semantics; prop defaults | QUICK |
| G4-13 | Themes | Eased text layers differ slightly from the published hex (D-106) | Accept, or fall back to `textStrong`; review with the user | — |
| G4-14 | Tooling | `cargo-nextest` and gitleaks are not installed locally | Install both | QUICK |

## M5 known gaps (open)
| ID | Area | Gap | Suggested fix | Size |
|---|---|---|---|---|
| G5-01 | Meals, grocery | Contributions of a deleted plan entry are parked for 6 h only; a late reconcile no longer sees them | A small `grocery_pending_source` table | QUICK |
| G5-02 | Meals API, grocery | `GroceryItemPatch` cannot express null | Double-option fields (absent keeps, null clears) | QUICK |
| G5-03 | Process | Kiosk calls are added without updating the LAN allow-list (`fa-api/src/lan.rs`); it happened again with `devices/elevate` (fixed 2026-11-23) | A unit test that walks the kiosk's API calls against `ALLOWED` | QUICK |
