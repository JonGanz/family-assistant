---
title: Repository review 2026-10-26 (process, code, direction, UI)
milestone: none
status: open
updated: 2026-10-26
arch: [Overall, Platform, Frontend, Ingest]
decisions: [D-023, D-050, D-057, D-061]
---

# Repository review, 2026-10-26

A 31-minute review requested by the user: (1) the human + AI development process, (2) bugs, security and API consistency, (3) whether the project is heading toward PROJECT.md, (4) the web UI. Each finding has an ID, a size (**QUICK** = a sub-agent can do it in under 10 minutes, **LARGER** = needs a brief or a question), and a status. Items marked `done` were fixed in this session.

## 3. Direction: are we heading the right way?

### Coverage of PROJECT.md
| Requirement | Status | Evidence |
|---|---|---|
| Everything local, local LLMs | planned (M4); no LLM code yet | README roadmap; no `fa-llm`/`fa-decide` crate exists |
| 32 GB / 8 GB budget | designed, not tested | arch/Overall.md deployment table |
| Deterministic first | done for M0–M3 | fa-rules, fa-pipeline (`on_unavailable: skip`) |
| Members with adult/child roles | done | fa-domain, MembersView |
| Interests/hobbies/responsibilities guide decisions | **partial: stored, never read** | fa-config household import; no consumer in fa-rules/fa-tasks |
| Gmail list polled and processed | done in code, unproven on a real account | fa-connectors imap/google, H-10 open |
| Categorise mail, ignore junk | done (rules only) | fa-pipeline presets |
| Spam → deals | descoped | D-012 |
| Bills itemised | done | extract_bills, BillsView, D-059 |
| School/daycare/teacher linking per child | done (rules) | D-055, D-056, org enrollments |
| Calendars (Google, CalDAV, ICS) | done in code; only ICS proven | H-05/H-06 open |
| Email events → calendar if missing | done, approval-gated | D-057 |
| Weather at a glance | done | fa-weather; advisories deferred to M4 |
| Household task lists, schedules, routines, email tasks | done | fa-tasks, D-037..D-049, D-060 |
| Homework with steps | partial (generic `steps`) | m2 migration |
| Fixed/subset/pool/smart assignment | partial; smart = M6 | D-039, D-040, D-050 |
| Meal plan, recipe extraction, grocery | planned (M5) | no code |
| Echo Show kept open | in code, unproven on device | kiosk/keepAlive.ts, D-064 |
| Browser | done | Vue PWA |
| TUI | planned (M8) | D-050 |

### Risks
- **R-01** Nothing has run on real data. Four milestones are "code-complete" but every human step H-02..H-11 is open, although D-023 says each milestone ends with a demo on real hardware.
- **R-02** The Echo Show, the main interface, has never been tested. D-064 (plain-HTTP LAN listener) exists because Silk cannot use Tailscale; keep-alive, SSE and the service worker on Silk are unverified.
- **R-03** The approval inbox may add load. M3 defaults: every event, most bills, every email task and every contact needs approval, with a push per adult. Fixtures already show false positives (plan/M3.md "Known gaps").
- **R-04** H-11 asks for a week of manual labelling before M4 starts: a load cost paid before any AI benefit.
- **R-05** The M4 design (backend registry, sysprobe, GPU agent with gaming detection, fa-eval, kNN, two System One sizes, Ollama) is platform-scale for one household.
- **R-06** Food (M5) is concrete daily value that needs no GPU, yet waits behind M4.
- **R-07** Member interests/hobbies/responsibilities are write-only data.

### Recommendations (for the user to decide; each would become a question)
1. **M3.5 real-hardware shakedown before M4**: do H-02..H-11 on the real server, one real mail source, one Google and one CalDAV calendar, the Echo Show via D-064, ntfy on phones.
2. **Measure approval load during the shakedown** (proposals/day, approve/reject ratio) and fix the M3 known gaps first, so M4 scope comes from measured triage burden.
3. **Consider M5 (food) before or alongside M4**: schema.org Recipe parsing is deterministic.
4. **Cut M4 to a minimal slice**: one System One backend and one Ollama model behind the existing pipeline stages, aimed at the decision points with the most rejections; defer the registry, profile suggester, gaming detection and full eval until a second hardware profile exists.
5. **Name a consumer for interests/hobbies/responsibilities** (relevance tags in fa-rules, M6 smart-assignment inputs).

## 1. Process: human + AI development loop

Verdict: the process works well where it is written down. All 193 managed files conform to the front-matter rules (checked exhaustively), decisions are numbered and cross-linked, and the e2e and fresh-Postgres rules have caught real bugs. The weak spots are everything after implementation: index dates, log closure and human-step tracking drift because nothing checks them; the repeated milestone orchestration (plan → waves → integrate → close) runs from memory rather than from skills; and the user has no single "what do I owe the project" page.

| ID | Finding | Fix | Size | Status |
|---|---|---|---|---|
| P-01 | `wipe-and-run.sh` ran `docker compose down -v`, deleting the dev DB volume that may hold the real household, with a hard-coded home path and an orphaned fixture server | Moved to `scripts/demo-reset.sh` with a typed `wipe` confirmation (`FA_DEMO_RESET_YES=1` to skip), `trap 'kill 0'`, `just demo-reset`, README pointer | QUICK | done |
| P-02 | No single "what next" page: open Qs, H-01..H-11 status, decisions waiting on the user (RUSTSEC-2026-0192), M3 known gaps are in four places | Add `plan/human-steps.md` (one row per H-step with status) and a `fa.py status` command that writes `STATUS.md` | LARGER | done (M3.5) |
| P-03 | Index `updated:` dates were stale in log, design, decisions and arch indexes | Stamped; `fa.py` writers should stamp `today` (part of P-06) | QUICK | done |
| P-04 | The 2026-10-26 log was never closed: D-064 "in progress", "still to do" list left after commit 141b07a | Closed out in today's log entry; rule added to CLAUDE.md | QUICK | done |
| P-05 | No front-matter lint, CI does not check the design tree | `python3 tools/fa.py check` added (required keys, status enums, id/filename match, duplicate numbers, index ⇄ file parity); wired into `ci.yml` and `scripts/ci-local.sh` | QUICK | done |
| P-06 | `tools/fa.py` is a library, not a CLI; the skill asks agents to write `python3 -c` one-liners whose positional args silently corrupt files | Add argparse subcommands `new-q`, `new-d`, `resolve`, `supersede`, `log`, `status` with keyword flags | LARGER | todo |
| P-07 | Only one skill (`process-answers`) while every milestone repeats plan → `_rules.md` → briefs → cross-check → waves → integrate → close | Add `plan-milestone`, `run-wave`, `close-milestone` skills | LARGER | todo |
| P-08 | No milestone-closing checklist; M0 and M1 lack `## Outcome`; H-step status is nowhere | Checklist added to CLAUDE.md; backfill M0/M1 Outcome sections | QUICK | done (M3.5) |
| P-09 | `plan/M*/_rules.md` is copied and hand-edited per milestone | Extract `plan/_rules-common.md`, keep only deltas per milestone | QUICK | done (M3.5) |
| P-10 | 5 of 15 M3 briefs cite no decision | Add `decisions:` to task front-matter and require it in `fa.py check` | QUICK | done (M3.5) |
| P-11 | D-030 `affects` Platform.md but Platform.md never mentions it | Reference added next to D-034 | QUICK | done |
| P-12 | "Only answers become D-records" has an undocumented exception (D-064 from a user instruction) | Rule amended in CLAUDE.md | QUICK | done |
| P-13 | No review workflow: this session was ad hoc | `review` section added to CLAUDE.md: time-boxed, findings to `plan/reviews/`, decisions needed become questions | QUICK | done |
| P-14 | CLAUDE.md mixes process rules with implementation trivia (ts-rs, `Date.parse`); disk thresholds disagree between CLAUDE.md (15 GB deps) and `_rules.md` (6 GB free) | Move gotchas to `plan/_rules-common.md` (with P-09); unify the disk rule | QUICK | done (M3.5) |

## 2. Code: bugs, security, API consistency, extensibility

Coverage: fa-api (lib, extract, routes, hub, parts of mail and feeds), fa-auth (pin, login, resolve, limiter, devices, parts of oauth), fa-domain authz, fa-store secret and repos, fa-connectors http. Not covered: fa-mail IMAP/Gmail internals, ICS/recipe fetch beyond http.rs, most of tasks.rs, notify webhooks. Clean: no SQL injection (all `format!` splices are constant column lists), OAuth uses PKCE with single-use state, device tokens are 32 random bytes stored hashed, argon2id with dummy verify, CSRF header plus HttpOnly/SameSite=Lax cookie, avatars checked by magic bytes, internal errors are opaque.

| ID | Sev | Finding | Fix | Size | Status |
|---|---|---|---|---|---|
| C-01 | HIGH | `/device/elevate` has no rate limiter and `pin_lockout` is read without `FOR UPDATE`, so a kiosk token holder can brute-force an adult PIN in parallel (`fa-auth/src/pin.rs:76`, `fa-store/src/repo/member.rs:262`) | Per-device limiter in `AuthService::elevate` (done this session as a process-wide `OnceLock` with a fixed 5 per 15 min budget; making it an `AuthConfig` field is a follow-up); `SELECT … FOR UPDATE` needs a `.sqlx` regen | QUICK + LARGER | done (M3.5) |
| C-02 | MED | Login lockout has the same read-then-update race (`fa-store/src/repo/account.rs:167`) | `FOR UPDATE` plus a conditional update | QUICK (needs sqlx prepare) | done (M3.5) |
| C-03 | MED | Own-password change needs no current password and keeps other sessions alive (`fa-api/src/routes.rs:663`) | Require `current_password`, revoke other sessions; DTO + web change | LARGER | done (M3.5) |
| C-04 | MED | Default trusted-proxy list includes all of `172.16.0.0/12`, so any bridge peer can spoof `X-Forwarded-For` (`fa-api/src/extract.rs:147`) | Default to loopback plus the detected Docker gateway; make the list a setting | QUICK (decision: affects Docker deployments) | done (M3.5) |
| C-05 | MED | Outbound HTTP client checks only the scheme: SSRF via calendar-source test URLs to loopback/private/link-local, through redirects (`fa-connectors/src/http.rs:24`) | Private-network denylist with a custom resolver, `allow_private_hosts` toggle (default off) for local CalDAV, same check in the redirect policy | LARGER | done (M3.5) |
| C-06 | MED | SSE `RecvError::Lagged` dropped silently; slow clients go stale (`fa-api/src/routes.rs:944`) | Emit `resync` | QUICK | done |
| C-07 | MED | Replay never signals when `Last-Event-ID` is outside the ring or ids restarted (`fa-api/src/hub.rs:66`) | Prepend `resync` | QUICK | done |
| C-08 | LOW | Unelevated devices (LAN listener included) can read the approval inbox (`fa-domain/src/authz.rs` ReadInbox) | Confirm intended under D-064, else per-device setting | QUICK (question) | done (M3.5) |
| C-09 | LOW | Anyone, children and devices included, can dismiss weather advisories | Configurable, default adults + elevated devices | QUICK | done (M3.5) |
| C-10 | LOW | AES-GCM secrets carry no AAD, so rows can be swapped between owners by someone with DB write access (`fa-store/src/secret.rs`) | Bind AAD to household/owner/kind; re-encryption migration | LARGER | todo |
| C-11 | LOW | Login limiter fails open on a poisoned mutex and skips when the IP is unknown (`fa-auth/src/limiter.rs:24`) | `into_inner()`, global fallback bucket | QUICK | done |
| C-12 | LOW | argon2 verify runs while a pooled transaction is open; a burst exhausts the pool (`login.rs:75`, `pin.rs:76`) | Read, release, verify, short transaction to record | QUICK | done (M3.5) |
| C-13 | LOW | Route naming drifts: `/device/elevate` vs `/devices`, `/away/{id}` vs `/members/{id}/away`, three dry-run verbs (`validate`, `validate`, `test`), kind in the path for `/inbox/{kind}/{id}/approve` | Settle conventions, document in Platform.md API table, alias old paths for one milestone | QUICK (doc) + LARGER | todo |
| C-14 | LOW | Only `GET /messages` paginates; tasks, bills, orgs, inbox, sessions return bare arrays | Standard `{items, next_before}` envelope for growing lists | LARGER | todo |
| C-15 | LOW | Session cookie built by hand with `format!`; `cookie_secure=false` behind TLS goes unnoticed (`routes.rs:192`) | Use the `cookie` builder (needs `time` as a direct fa-api dependency for `max_age`); warn at startup in `bins/fa-server/src/serve.rs`, where `cookie_secure` is set | QUICK | done (M3.5) |

## 4. UI: consistency and navigation

Root causes: no shared page shell, no global UI kit (`base.css` has no button/input/table styles; `views/parts/settings.css` is the de facto kit but imported per view), and a flat 17-link top nav mixing value and settings pages.

| ID | Finding | Fix | Size | Status |
|---|---|---|---|---|
| U-01 | Flat nav interleaves value and settings pages; no Settings entry; no active style; no `aria-label`; no mobile behaviour | Primary nav (Today, Calendar, Tasks, Routines, Members, Mail, Inbox, Bills) + a right-side Settings button; a Settings sub-nav on `/settings/*`; `.router-link-active` styling; wrap at phone width; 40 px targets | QUICK | done |
| U-02 | Errors rendered four ways (`ErrorBanner`, `.settings-error`, scoped `.error`, bare `role=alert`) and often twice | One `ErrorBanner` everywhere (kiosk excepted) | QUICK | done |
| U-03 | Hard-coded hex colours in Login, Devices, Dashboard, PinPad, TaskItem; `#fff` breaks dark mode; danger/warning/success have no dark-mode values; `--min-touch` only exists in kiosk | Tokens, dark-mode values, `--min-touch: 44px` on `:root`, `--color-on-accent` | QUICK | done |
| U-04 | Duplicate titles (Mail vs Mail settings, Tasks vs Task settings, Weather vs Weather rules); Home h1 is the household name while the nav says Today | Titles aligned with nav labels | QUICK | done |
| U-05 | `settings.css` imported per view, so a page renders differently depending on which route loaded first | Imported once in `main.ts` | QUICK | done |
| U-06 | Route name `devices` breaks the `settings-*` pattern | Renamed `settings-devices` | QUICK | done |
| U-07 | No page shell: bare `<h1>` per view, actions placed three different ways, subtitles ad hoc | `components/PageHeader.vue` (`title`, `subtitle`, `back`, `eyebrow`, `actions` slot) added with unit tests; all 22 app views migrated | LARGER | done |
| U-08 | Settings pages are not structurally distinct; `/settings` redirects to Household; `/members` (CRUD) is a top-level page; `/settings/notifications` is the only user-level settings page | Nested `/settings` parent route with `SettingsLayout` (sidebar on desktop, chips on mobile), a `SettingsIndexView` of grouped cards (Personal / Household / Sources & rules / System); move Members to `/settings/members` with a redirect (needs a decision on what children see) | LARGER | done (M3.5) |
| U-09 | No UI kit: ~60 default-styled buttons, one-off classes (`primary`, `cancel`, `ok`, `undo`, `try`); `.row` defined 8 times, `.error` 5, `.actions` 3; three tab implementations | Kit added to `base.css` (`button`/`input` base, `.btn-*`, `.stack/.row`, `.form/.form-wide/.field/.field-inline`, `.muted/.text-*`, `.card/.card-list`, `.tabs`); views migrated off one-off classes; `.badge` still lives in `views/parts/settings.css` | LARGER | done (M3.5) |
| U-10 | Loading states almost absent (only the dashboard has one); empty states use three different classes | `components/StateBlock.vue` added with unit tests and used by the list views; the Tasks and Calendar stores expose no `loading` flag, so those show error/empty only | LARGER | done (M3.5) |
| U-11 | `base.css` declares `:root` tokens twice and the palettes three times; two kiosk font systems | Consolidated into one `:root`, one dark `@media` and one `data-theme=dark` block; kiosk font systems still two | QUICK | done (M3.5) |
| U-12 | Accessibility: no skip link, unlabelled `<section>` roots, nested pages don't mark their parent nav item, `--color-warning` borderline for small text | Skip link (done); `aria-labelledby` on sections (todo); `router-link-active` on parents (done by U-01); warning darkened to #9a5b00 (done) | QUICK | done (M3.5) |
| U-13 | Forms on value pages capped at 32 rem by `.settings-form` | `.form-wide` variant (with U-09) | QUICK | done (M3.5) |

## Suggested order of work
1. **Decisions needed from the user**: batch 17, Q-061 (milestone order), Q-062 (M4 scope), Q-063 (device inbox and advisories, C-08/C-09), Q-064 (trusted proxies, C-04), Q-065 (Members placement, U-08), Q-066 (interests consumer, R-07).
2. **Next code wave (one brief each, Sonnet 5.5 medium):** C-01/C-02 `FOR UPDATE` + sqlx prepare; C-12; C-15; C-05 SSRF guard; C-03 password change; U-07 PageHeader; U-09 UI kit; U-10 StateBlock; U-11; U-12.
3. **Process:** P-02 status page, P-06 fa.py CLI, P-07 skills, P-08 backfill, P-09/P-14 rules-common, P-10 brief decisions.
