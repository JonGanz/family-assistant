# Improvements backlog

Outstanding findings and suggestions from the 2026-10-26 repository review (full tables with status in [plan/reviews/2026-10-26-repo-review.md](plan/reviews/2026-10-26-repo-review.md)). Items already fixed in that session are not repeated here. Size: **QUICK** = a sub-agent brief under 10 minutes, **LARGER** = a milestone task or a question first.

## Decisions taken (batches 17 and 18, 2026-10-26)
| Q | Decision | Effect on this backlog |
|---|---|---|
| Q-061 | D-066: M3.5 = onboarding (D-065) + real-hardware shakedown, then M4 | R-01, R-03 scheduled in M3.5 |
| Q-062 | D-067: M4 minimal slice; registry, sysprobe, GPU agent, full eval in M4.1 | R-05 settled |
| Q-063 | D-068: dashboard at a glance only (no inbox, no mail); advisories not dismissable, they clear themselves | C-08, C-09 fixed by removal in M3.5 |
| Q-064 | D-069: trusted proxies loopback only; compose sets the sidecar address | C-04 in M3.5 |
| Q-065 | D-070: Members on an adult-only Settings screen, no redirect; children see members on the dashboard | U-08 in M3.5 |
| Q-066 | D-071: interests, hobbies, responsibilities feed LLM features only (M4+) | R-07 accepted as by design |
| Q-067 to Q-072 | D-072 to D-077: whole app over HTTP, open first-run wizard, generated secrets, settings in the UI, one container, GHCR amd64 | onboarding, see [plan/M3.5.md](plan/M3.5.md) |

**Where each item goes:** [plan/M3.5.md](plan/M3.5.md), workstream E, takes C-01b, C-02, C-03, C-04, C-05, C-08, C-09, C-12, C-15, U-08, U-09b to U-16, P-02, P-08b, P-09, P-10, P-14 to P-16. **Deferred to the M4 plan:** C-10, C-13, C-14, P-06, P-07, and the second code review of the unreviewed crates.

## Direction (section 3 of the review)
- **R-01** *(M3.5 shakedown, D-066)* Nothing has run on real data: human steps H-02..H-11 are all open, so "code-complete" is not done. Recommendation: an M3.5 shakedown on the real server (one real mail source, one Google and one CalDAV calendar, the Echo Show via D-064, ntfy on phones).
- **R-02** *(M3.5 shakedown; D-072 keeps the whole app on HTTP, so the Silk test covers the full app too)* The Echo Show, the main interface, is unproven on Silk over plain HTTP (keep-alive, SSE, service worker).
- **R-03** *(M3.5 shakedown, D-066; the inbox is app-only per D-068)* The approval inbox may add load: M3 approves nothing automatically and fixtures already show false positives (plan/M3.md "Known gaps"). Measure proposals per day and approve/reject ratio during the shakedown before sizing M4.
- **R-04** *(decided during the shakedown week)* H-11 asks for a week of manual labelling before M4; consider shrinking it.
- **R-05** *(settled by D-067: minimal slice, M4.1 for the rest)* The M4 design (backend registry, sysprobe, GPU agent with gaming detection, eval suite, kNN, two System One sizes, Ollama) is platform-scale for one household.
- **R-06** *(D-066: M5 stays after M4)* Food (M5) is deterministic daily value and could run before or beside M4.

## Code: security and correctness
| ID | Sev | Finding | Fix | Size |
|---|---|---|---|---|
| C-01b ✅ done 2026-10-26 (M3.5) | HIGH | `pin_lockout` is read without `FOR UPDATE`, so parallel guesses pass the check before a failure is recorded (`fa-store/src/repo/member.rs:262`). The per-device limiter added in the review mitigates it; it is a process-wide `OnceLock` with a fixed 5 per 15 min budget | `SELECT … FOR UPDATE` plus `cargo sqlx prepare`; move the limiter budget into `AuthConfig` | QUICK |
| C-02 ✅ done 2026-10-26 (M3.5) | MED | Login lockout has the same read-then-update race (`fa-store/src/repo/account.rs:167`) | `FOR UPDATE` plus a conditional update | QUICK |
| C-03 ✅ done 2026-10-26 (M3.5) | MED | Own-password change needs no current password and leaves other sessions alive (`fa-api/src/routes.rs:663`) | Require `current_password`, revoke other sessions; DTO and web change | LARGER |
| C-04 ✅ done 2026-10-26 (M3.5) | MED | Default trusted proxies include `172.16.0.0/12` (`fa-api/src/extract.rs:147`) | Loopback only; compose sets the sidecar address (D-069) | QUICK |
| C-05 ✅ done 2026-10-26 (M3.5) | MED | Outbound HTTP client checks only the scheme: SSRF through calendar-source test URLs to loopback, private and link-local addresses, through redirects (`fa-connectors/src/http.rs:24`) | Private-network denylist with a custom resolver, `allow_private_hosts` toggle (default off, a Settings, System field per D-075) for local CalDAV, same check in the redirect policy | LARGER |
| C-08 ✅ done 2026-10-26 (M3.5) | LOW | Unelevated devices can read the approval inbox (`fa-domain/src/authz.rs`) | Refuse `ReadInbox` for all devices; remove the dashboard inbox panel (D-068) | QUICK |
| C-09 ✅ done 2026-10-26 (M3.5) | LOW | Anyone can dismiss weather advisories | Remove dismiss; advisories clear when the rule stops matching or `valid_until` passes (D-068) | QUICK |
| C-10 | LOW | AES-GCM secrets carry no AAD, so rows can be swapped between owners by someone with DB write access (`fa-store/src/secret.rs`) | Bind AAD to household, owner and kind; re-encryption migration | LARGER |
| C-12 ✅ done 2026-10-26 (M3.5) | LOW | argon2 verify runs inside an open pooled transaction; a burst exhausts the pool (`login.rs:75`, `pin.rs:76`) | Read, release, verify, then a short transaction to record | QUICK |
| C-15 ✅ done 2026-10-26 (M3.5) | LOW | Session cookie built by hand with `format!`; `cookie_secure=false` behind TLS goes unnoticed (`routes.rs:192`) | Use the `cookie` builder; `Secure` becomes per-request (`FA_COOKIE_SECURE=auto`, D-065), which replaces the startup warning | QUICK |

## Code: API consistency and extensibility
| ID | Finding | Fix | Size |
|---|---|---|---|
| C-13 | Route naming drifts: `/device/elevate` vs `/devices`, `/away/{id}` vs `/members/{id}/away`, three dry-run verbs (`validate`, `validate`, `test`), kind in the path for `/inbox/{kind}/{id}/approve` | Settle conventions in the Platform.md API table, alias old paths for one milestone | QUICK doc, LARGER code |
| C-14 | Only `GET /messages` paginates; tasks, bills, orgs, inbox and sessions return bare arrays | Standard `{items, next_before}` envelope for lists that grow | LARGER |
| — | Review coverage gaps: fa-mail IMAP/Gmail internals, ICS and recipe fetch beyond `http.rs`, most of `tasks.rs`, notify webhooks were not reviewed | Schedule a second code review on those crates | — |

## Front end
| ID | Finding | Fix | Size |
|---|---|---|---|
| U-08 ✅ done 2026-10-26 (M3.5) | Settings pages are not structurally grouped; `/settings` redirects to Household; `/members` is top-level | Nested `/settings` parent route with a `SettingsLayout` (sidebar on desktop, chips on mobile) and a `SettingsIndexView` of grouped cards (Personal, Household, Sources and rules, System); Members to `/settings/members`, adult-only, **no redirect** (D-070); System hosts the runtime settings (D-075) | LARGER |
| U-09b ✅ done 2026-10-26 (M3.5) | 88 uses of the old `settings-*` classes remain in `views/parts/*` and `components/mail/*`, served by aliases in `views/parts/settings.css`; `.badge` is not in the kit | Migrate to `.form/.muted/.card-list/.text-*`, add `.badge` to `base.css`, delete `settings.css` | QUICK |
| U-10b ✅ done 2026-10-26 (M3.5) | The Tasks and Calendar stores expose no `loading` flag, so those pages show only error and empty states | Add `loading` to both stores and pass it to `StateBlock` | QUICK |
| U-11b ✅ done 2026-10-26 (M3.5) | Two kiosk font systems (`--kiosk-font-*` and `.kiosk { font-size: 28px }`) | Keep one | QUICK |
| U-12b ✅ done 2026-10-26 (M3.5) | View roots are unlabelled `<section>`s; nested pages (`/tasks/:id`, `/mail/:id`) do not mark their parent nav item | `aria-labelledby` on sections; match nav active state on path prefix | QUICK |
| U-14 ✅ done 2026-10-26 (M3.5) | Tabs use the kit styling but no arrow-key handling | Shared `Tabs.vue` with roving tabindex | QUICK |
| U-15 ✅ done 2026-10-26 (M3.5) | No mobile menu: the primary nav wraps rather than collapsing below 40 rem | Scroll row or hamburger with the Settings button kept visible | QUICK |
| U-16 | The new nav, page shell and kit were verified by lint, tests, typecheck and a production build, not in a browser or on the Echo Show | Visual pass on phone width, desktop and the kiosk | — |

## Process and tooling
| ID | Finding | Fix | Size |
|---|---|---|---|
| P-02 ✅ done 2026-10-26 (M3.5 T-01) | No single "what should I do next" page: open questions, H-01..H-11 status, decisions waiting (RUSTSEC-2026-0192), M3 known gaps live in four places | `plan/human-steps.md` with one row per H-step and status, plus `fa.py status` writing `STATUS.md` | LARGER |
| P-06 | `tools/fa.py` is a library; the skill asks agents for `python3 -c` one-liners whose positional args silently corrupt files | argparse subcommands `new-q`, `new-d`, `resolve`, `supersede`, `log`, `status` with keyword flags (`check` exists) | LARGER |
| P-07 | Only one skill (`process-answers`) while each milestone repeats plan → rules → briefs → cross-check → waves → integrate → close | `plan-milestone`, `run-wave`, `close-milestone` skills | LARGER |
| P-08b ✅ done 2026-10-26 (M3.5 T-01) | M0 and M1 plans have no `## Outcome` section | Backfill | QUICK |
| P-09 ✅ done 2026-10-26 (M3.5 T-01) | `plan/M*/_rules.md` is copied and hand-edited per milestone | `plan/_rules-common.md` plus per-milestone deltas | QUICK |
| P-10 ✅ done 2026-10-26 (M3.5 T-01) | 5 of 15 M3 briefs cite no decision | `decisions:` key in task front-matter, enforced by `fa.py check` | QUICK |
| P-14 ✅ done 2026-10-26 (M3.5 T-01) | CLAUDE.md mixes process rules with implementation trivia (ts-rs, `Date.parse`); disk thresholds differ between CLAUDE.md and `_rules.md` | Move gotchas to `_rules-common.md`; one disk rule | QUICK |
| P-15 ✅ done 2026-10-26 (M3.5 T-01) | `fa.py check` has no negative test and `ci.yml` was not YAML-validated after the `docs` job was added | Add a bad-fixture test; run the workflow once | QUICK |
| P-16 ✅ done 2026-10-26 (M3.5) | Not run during the review: `scripts/ci-local.sh`, `scripts/e2e.sh`, `scripts/compose-smoke.sh`, the DB-backed Rust tests | Run before the next milestone starts | — |

## Suggested order
1. ~~Answer Q-061 to Q-066~~ (done: D-066 to D-071).
2. ~~M3.5 code~~ (done 2026-10-26: every item listed for it above is closed; see [plan/M3.5.md](plan/M3.5.md), *Outcome*).
3. The M3.5 shakedown (R-01 to R-04, U-16).
4. The M4 plan picks up the deferred items: C-10, C-13, C-14, P-06, P-07, and the second code review.
