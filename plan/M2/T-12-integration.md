---
id: T-12
title: "Integration: e2e with ntfy, specs, compose and env, deploy docs, README"
milestone: M2
wave: 6
depends_on: [T-10, T-11]
agent: {model: sonnet, effort: medium}
status: done
owns: ["e2e/**", "apps/web/e2e/**", "scripts/**", "deploy/**", ".github/workflows/**", "README.md", "justfile", ".env.dev"]
---

# T-12: Integration: e2e with ntfy, specs, compose and env, deploy docs, README

You are a sub-agent implementing one task of the M2 milestone (Tasks) of the Family Assistant project (a self-hosted family organiser: a Rust backend and a Vue frontend).

## Read first
- `plan/M2/_rules.md` (mandatory ground rules)
- `plan/M2.md`: *Goal* (the definition of done), *Process config*, *Household TOML additions*, *Notifications*, *Human steps*
- `arch/Tasks.md` §5.2 (ntfy deployment), `arch/Frontend.md` §3a and §4
- The M1 integration pieces: `scripts/e2e.sh`, `scripts/ci-local.sh`, `scripts/compose-smoke.sh`, `e2e/` (seed.mjs, specs, stubs), `apps/web/e2e/`, `deploy/compose.example.yaml`, `deploy/compose.dev.yaml`, `deploy/.env.example`, `deploy/README.md`, `.github/workflows/ci.yml`, `README.md`, `justfile`, `.env.dev`
- *Requested README changes* below (from the earlier tasks' reports)

## You own
The paths in the front-matter `owns` list.

## Work
1. **e2e stack (`scripts/e2e.sh`):** add a throwaway ntfy container (`binwiederhier/ntfy:v2.17.0`, `serve`, a free port, removed with `docker rm -f -v` in the trap), and pass `FA_NTFY_URL` to `serve api` and `serve worker` (still two separate processes).
2. **Seed (`e2e/seed.mjs`):** fictional lists, a rotation chore (`every_days`), a weekly `rrule` chore, a pool chore, a homework task with a key date, a routine on `daily` with three steps, notify prefs for an adult and a child (copy to the adult), all through the API or a TOML import. Use `fa-server tasks run-now` (or wait for the minute jobs) to materialise.
3. **API specs (`e2e/tasks.spec.ts`):** materialisation, rotation after completion, undo, claim, child scoping, device completion with and without `member`, the Now view, the calendar range, routine ticking to done, a weather task rule (the Open-Meteo stub is always freezing and rainy) creating one task per day, and **notifications reaching ntfy**: read back `GET {ntfy}/{topic}/json?poll=1&since=all` and check the `assigned` message, a batched "N updates" message, the child copy on the adult's topic, and the test endpoint. Quiet hours: set the member's quiet hours to cover now and check nothing arrives, then clear them.
4. **UI specs (`apps/web/e2e/m2.spec.ts`):** create a definition in Settings → Tasks and see its task in the Tasks page; complete and undo; the dashboard `now` panel completion through the member picker; the routine panel progress bar reaching 100%; Settings → Notifications "Send test" succeeds.
5. **Compose and env:** `deploy/compose.example.yaml` pins `binwiederhier/ntfy:v2.17.0` and passes `FA_NTFY_URL` (internal service URL) and optional `FA_NTFY_TOKEN` to fa-server; `deploy/.env.example` documents both; `deploy/compose.dev.yaml` gains an `ntfy` service on `127.0.0.1:18090` so local runs can notify; `.env.dev` sets `FA_NTFY_URL=http://127.0.0.1:18090`. Keep `scripts/compose-smoke.sh` green.
6. **deploy/README.md:** an "ntfy notifications" section: exposing ntfy on the tailnet (`tailscale serve`), the phone app setup with the subscribe URL or QR, optional access control (`auth-default-access: deny-all`, a write token for fa-server, per-topic read access with `ntfy access`), and the H-08 steps.
7. **README.md** (user-facing; verify every step by running it):
   - Status line and Roadmap table: M2 code-complete (M6 smart assignment, M7 goals, M8 TUI are already there).
   - "Run it locally": start the dev ntfy with the database, and a new step "Tasks and routines" (create a list and a chore in the UI, run `cargo run -p fa-server -- tasks run-now` to see it immediately, check it off, see the dashboard panels; subscribe to notifications by opening the subscribe URL in a browser at `http://127.0.0.1:18090/<topic>` and pressing "Send test").
   - A "Tasks, routines and notifications" section explaining lists, schedules (after-done vs repeats), missed behaviour, rotation/pool/fixed, absences, routines and school days, the Now list, calendar display (key dates), and notifications (triggers, quiet hours, children's copies).
   - Configuration reference: `FA_NTFY_URL`, `FA_NTFY_TOKEN`, and the new TOML sections with an example.
   - Repository layout: `crates/fa-tasks`, `crates/fa-notify`.
8. **CI:** make sure `ci.yml` and `scripts/ci-local.sh` cover the new crates (they likely do through `--workspace`; check the hygiene steps).

## Acceptance
- [ ] `scripts/e2e.sh` passes (quote the spec counts from the output)
- [ ] `scripts/ci-local.sh` passes
- [ ] `scripts/compose-smoke.sh` passes
- [ ] The README's new and changed steps were run as written (say which), **on a throwaway stack, not the dev DB's real data**
- [ ] No anonymous Docker volumes leaked (`docker volume ls -qf dangling=true | wc -l` before and after)

## Requested README changes
- From T-09: new CLI commands `fa-server notify test --member <key>` (publishes a test notification, prints the subscribe URL; clear error when `FA_NTFY_URL` is unset) and `fa-server tasks run-now` (one pass of `tasks.materialize`, `tasks.tick`, `routines.materialize`); both accept the global `--household <id>`. `config import` now fails before writing if any RRULE, weather condition, or `season_condition` is invalid, listing every error. `FA_PUBLIC_BASE_URL` is also used for notification click links. `POST /members/{id}/notify-prefs/test` returns 204.
- Task completion, reopen, and skip are written to the audit log (D-013).

- From T-06: document `FA_NTFY_URL` (optional; without it notifications are off and the UI says so), `FA_NTFY_TOKEN` (optional bearer secret), that plain `http://` ntfy URLs are allowed (independent of `FA_ALLOW_HTTP_FEEDS`), and that `notify.send` runs every minute. Click links use `FA_PUBLIC_BASE_URL`.
- From T-08: the TOML reference below came from fa-config's tests. **Its conditions (`temp_min > 5`, `rain_mm > 5`) are placeholders, not valid weather DSL**: replace them with real DSL from `arch/Feeds.md` §4 (for example `min(temp_c, today) > 5`, `sum(precip_mm, next 24h) > 5`) and check them with `config import` before publishing. Replace `School/Term dates` with a fictional source/calendar from the example household if one exists.
```toml
[household]
now_list_size = 5
now_weights = { due = 1.0, flag = 0.75, weather = 0.5 }
quiet_from = "20:30"
quiet_until = "06:45"

[[members]]
key = "riley"
display_name = "Riley"
role = "child"
[members.notify]
triggers = ["assigned", "due_soon"]   # assigned|overdue|digest|due_soon|routine_nudge|source_reauth
due_soon_minutes = 90
nudge_minutes = 5
quiet_from = "21:30"
quiet_until = "07:30"
copy_to = "alex"                      # adult member key

[[task_lists]]
name = "Chores"
kind = "chores"                       # chores|homework|general
claim_ttl_minutes = 60                # omitted: until end of local day
color = "#44aa88"

[[task_definitions]]
list = "Chores"
title = "Take out the bins"
steps = ["Recycling", "Rubbish"]
rrule = "FREQ=WEEKLY;BYDAY=TU"        # or every_days = 3 (exactly one)
start_date = "2026-01-06"
due_time = "19:00"
lead_days = 1
candidates = ["alex", "riley"]        # member keys, in order
strategy = "rotation"                 # fixed|rotation|pool
season_from = "04-01"
season_until = "10-31"
season_condition = "temp_min > 5"     # PLACEHOLDER: replace with valid DSL

[[routines]]
name = "Morning"
members = ["riley"]
starts_at = "07:00"
done_by = "08:00"
day_filter = "school_days"
school_calendar = "School/Term dates" # "source name/calendar name"
no_school_match = ["holiday", "inset"]
steps = [{ text = "Brush teeth", target_time = "07:10" }, { text = "Pack bag" }]

[[weather_rules]]
name = "Bins in the rain"
condition = "rain_mm > 5"             # PLACEHOLDER: replace with valid DSL
action = "task"
severity = "info"
message = "Rain tomorrow"
task_mode = "suggest"                 # create (default)|suggest
task_definition = "Take out the bins" # title, or "List/Title" if ambiguous
```

## Wave 5 outcomes (read before starting)
- T-10 (app shell) and T-11 (dashboard) are done; web tests 176 passed. Routes: `/tasks`, `/tasks/:id`, `/routines`, `/settings/tasks?tab=lists|definitions|routines|absences`, `/settings/notifications`. Dashboard panels `now`, `member_tasks`, `routine`; completion opens "Who did it?" then an undo toast.
- Main session: `GET /dashboard` now carries `household.now_list_size`, `household.completion_override`, and `task_lists`, and the dashboard store uses them.
- Not exercised by hand yet (cover them in your UI specs where feasible): the Routines page with real data, Absences, the routine editor, claim/release, Suggested accept/dismiss, the weather-rule task editor, and the full dashboard add flow (title → who-adds picker → `POST /tasks`).
- Run a real-stack kiosk check at 1920×1080 and portrait in your UI specs (screenshots into `apps/web/e2e` artifacts are fine; don't commit images).
