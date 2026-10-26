---
id: T-15
title: "Integration: e2e with greenmail, specs, compose, docs, README"
milestone: M3
wave: 7
depends_on: [T-13, T-14]
agent: {model: sonnet, effort: medium}
status: done
owns: ["e2e/**", "apps/web/e2e/**", "scripts/**", "deploy/**", ".github/workflows/**", "README.md", "justfile", ".env.dev", "apps/web/src/** (small bug fixes found by the specs only)"]
---

# T-15: Integration: e2e with greenmail, specs, compose, docs, README

You are a sub-agent implementing one task of the M3 milestone (Ingest: email, rules pipeline, bills, linking, approvals) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M3/_rules.md` (mandatory ground rules)
- `plan/M3.md`: *Goal* (definition of done), *Process config*, *Household TOML additions*, *Human steps*
- `arch/Ingest.md`; `arch/Frontend.md` §3b
- The M2 integration pieces: `scripts/e2e.sh`, `scripts/ci-local.sh`, `scripts/compose-smoke.sh`, `e2e/` (seed.mjs, specs, stubs), `apps/web/e2e/`, `deploy/*`, `.github/workflows/ci.yml`, `README.md`
- *Requested README changes* below (from the earlier tasks' reports)

## You own
- `e2e/**`
- `apps/web/e2e/**`
- `scripts/**`
- `deploy/**`
- `.github/workflows/**`
- `README.md`
- `justfile`
- `.env.dev`

## Work
1. **e2e stack (`scripts/e2e.sh`):** add a throwaway `greenmail/standalone:2.1.8` container (IMAP 3143 + SMTP 3025 on free ports; removed with `docker rm -f -v` in the trap) and pass `FA_ALLOW_PLAIN_IMAP=true` to `serve api` and `serve worker`. Add a writable CalDAV target for event writes: extend `e2e/stubs/server.mjs` with a minimal CalDAV calendar that accepts `PUT`/`DELETE` of `.ics` and answers the discovery/REPORT calls the M1 CalDAV client makes (look at `crates/fa-connectors/src/caldav.rs`), so approving an event really writes and the next sync shows it once.
2. **Seed:** fictional org (`example.edu` school with a teacher, an `example.com` biller), enrollments, child tags and gender, an IMAP mail source (read-only false, household visibility) for an adult; then put fixture mails into greenmail over SMTP (`e2e/fixtures/mail/*.eml`, all fictional): a teacher ICS invite, a newsletter with grade sections, a bill, a payment-received mail, a permission slip, a promotion, an unknown teacher at the school domain.
3. **API specs (`e2e/mail.spec.ts`):** sync (`fa-server mail sync-now` or the sync endpoint + polling), classification and member links, the event proposal in the inbox, approve → written to the CalDAV stub and appears once after a calendar sync, reject, bill → pay task on the adult pool → complete → bill paid → reopen → open, suggested task accept, contact suggestion accept, a label fix creates a human decision, auto label action applied on greenmail (IMAP keyword or folder — check with an IMAP client in the spec or via the API's actions list) and undo, visibility (other adult vs child vs device), `approval_needed` reaching ntfy for a `batched` adult, and **nothing** for a `digest` adult.
4. **UI specs (`apps/web/e2e/m3.spec.ts`):** add an IMAP source in Settings → Mail with Test connection; the Mail list and a message with a child filter; approve an event from the Inbox; labelling view one-tap confirm and progress; the dashboard `inbox` panel count and an elevated approve.
5. **Compose and env:** `deploy/.env.example` documents `FA_GOOGLE_API_BASE` (not for production), notes that `FA_ALLOW_PLAIN_IMAP`/`FA_IMAP_TLS_INSECURE` are test-only; `compose-smoke.sh` stays green.
6. **deploy/README.md:** a "Mail" section: Gmail API setup (enable the Gmail API, add scopes to the consent screen, Testing-mode 7-day re-consent), IMAP with app passwords (2-step verification, `imap.gmail.com:993`, `[Gmail]/All Mail`), read-only vs writable, privacy and retention, and the H-10/H-11 steps.
7. **README.md** (verify every step by running it on a throwaway stack, not the dev DB's data): status line and Roadmap (M3 code-complete); "Run it locally" gains a mail step (a local greenmail in `deploy/compose.dev.yaml` on 127.0.0.1 so a developer can add an IMAP source with plain IMAP and send a test mail by SMTP); a "Mail, inbox and bills" section (sources, what is read, categories, orgs and enrollments, the inbox and why everything waits for approval in M3, bills and pay tasks, actions and undo, labelling, privacy); configuration reference (new env vars, the new TOML sections with an example); repository layout (`fa-pipeline`, `fa-rules`, `fa-mail`); new CLI commands.
8. **CI:** `ci.yml` and `scripts/ci-local.sh` run the greenmail-gated tests too (start greenmail, set `FA_TEST_GREENMAIL`), and still use a fresh Postgres.

## Acceptance
- [ ] `scripts/e2e.sh` passes (quote the spec counts from the output)
- [ ] `scripts/ci-local.sh` passes
- [ ] `scripts/compose-smoke.sh` passes
- [ ] The README's new and changed steps were run as written (say which), on a throwaway stack
- [ ] No anonymous Docker volumes leaked (`docker volume ls -qf dangling=true | wc -l` before and after)

## Requested README changes
- From T-02: the IMAP integration tests: `docker run -d --name fa-t02-greenmail -p 33143:3143 -p 33993:3993 greenmail/standalone:2.1.8`, `FA_TEST_GREENMAIL=127.0.0.1:33143:33993 cargo test -p fa-connectors --test imap -- --include-ignored`, `docker rm -f -v fa-t02-greenmail` (exact commands in the doc comment of `crates/fa-connectors/src/imap.rs`).
- From T-08: env vars `FA_ALLOW_PLAIN_IMAP`, `FA_IMAP_TLS_INSECURE` (tests/e2e only) and `FA_GOOGLE_API_BASE` (stubs). New household TOML sections (`[household]` currency, gender_terms, disabled_signals, bill_lead_days, `[household.events]`, `[household.inbox]`, `[retention.mail]`; members aliases, gender, relevance_tags, `[[members.enrollments]]`, `notify.approval_mode`; `[[orgs]]` + `[[orgs.contacts]]`; `[[mail_sources]]`; `[[categories]]` (only those differing from the defaults are exported); `[[mail_rules]]`); calendar references are `"<calendar source key>/<remote_id>"`; **Gmail sources are not created by import** (connect them in Settings → Mail); IMAP sources import as `needs_reauth` without a password. See `config/examples/household.example.toml`.

## Wave 5–6 outcomes (read before starting)
- Backend (T-12): all routes of the plan; inbox approve/reject return 204; `GET /oauth/google/start?purpose=…`; calendar sources/calendars PATCH accept `writable` / `is_default_target` (CalDAV sources can be made writable directly; the e2e CalDAV stub needs that). CLI `fa-server mail sync-now [--source NAME]`, `fa-server mail process-now` (global `--household`); `config import` prints skipped Gmail sources. Member create/PATCH accept `aliases`, `gender`, `relevance_tags`.
- Web (T-13, T-14): routes `/mail`, `/mail/:id`, `/mail/label`, `/inbox`, `/bills`, `/settings/mail`, `/settings/orgs`; Settings → Household has a "Mail and approvals" group; Settings → Calendars has "Allow writing (approved events)" and per-calendar "Default target"; the dashboard `inbox` panel (not in default presets; add it through the layout editor or device settings API in the spec). The inbox badge follows SSE `inbox.updated` through `stores/live.ts`. Web tests: 213 passed. Nothing has been exercised in a browser yet: your UI specs are the first real run; fix small bugs you find in any `apps/web/src` file and list them in your report.
- Check the If-Match header format the server accepts (quoted `"<version>"` vs bare) for `apps/web/src/api/mailSettings.ts` (it sends a bare number); fix it there if the e2e shows 412/428.
