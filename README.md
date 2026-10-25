# Family Assistant

A self-hosted organiser that takes the mental load off a busy household. It pulls together the
family's email, calendars, weather, chores, homework, meals and groceries, and shows what matters
on a wall dashboard and on everyone's phone.

Everything runs locally. Deterministic rules do most of the work, and local AI models help only
where rules can't decide.

> **Status:** M2 (tasks) is code-complete. On top of the M0 platform (login, members, dashboard
> device pairing with PIN elevation, the job queue, backups, CI) and M1 (calendars: Google, CalDAV,
> ICS; the weather forecast with rule-based advisories; a real dashboard) it adds task lists,
> recurring chores with fixed, rotating or pooled assignment, routines, the "Now" list, weather
> and season tasks, tasks on the calendar, and ntfy push notifications. Feature milestones come
> next (see [Roadmap](#roadmap)).

## Contents
- [Run it locally](#run-it-locally)
- [Connecting calendars](#connecting-calendars)
- [Weather and the household location](#weather-and-the-household-location)
- [Tasks, routines and notifications](#tasks-routines-and-notifications)
- [The dashboard layout](#the-dashboard-layout)
- [Configuration reference](#configuration-reference)
- [Day-to-day development](#day-to-day-development)
- [Checks: run CI locally](#checks-run-ci-locally)
- [Deploying](#deploying)
- [Repository layout](#repository-layout)
- [How this project is designed](#how-this-project-is-designed)
- [Roadmap](#roadmap)
- [License](#license)

## Run it locally
This takes you from a fresh clone to the app running with the example household, a weather
forecast, an ICS calendar and the dashboard open in a browser. Every command below is meant to be
copied as-is, from the repository root.

### Prerequisites
| Tool | Version | Notes |
|---|---|---|
| Rust | per `rust-toolchain.toml` | `rustup` installs it automatically |
| Node.js | 24 | pnpm comes through **corepack** (`corepack pnpm@9 ...`); nothing needs a global install |
| Docker + Compose v2 | recent | dev Postgres, e2e tests, deployment |
| [`just`](https://github.com/casey/just) | optional | shortcuts only. Every recipe is a thin wrapper, so each step also shows the plain command |

### 1. Start the database and create a dev master key
```bash
docker compose -f deploy/compose.dev.yaml up -d --wait postgres ntfy
scripts/dev-key.sh
# with just: just dev-db dev-key
```
The dev database listens on `127.0.0.1:55432` (user, password and database are all `fa`). The dev
ntfy server (push notifications, step 9) listens on `127.0.0.1:18090`. The dev
key goes to `.dev/master.key` (git-ignored). The key encrypts stored secrets such as calendar
links, so it is needed by every `fa-server` command. Local settings live in [`.env.dev`](.env.dev).

### 2. Build the web app and load the settings
```bash
(cd apps/web && corepack pnpm@9 install --frozen-lockfile && corepack pnpm@9 run build)
```
`fa-server` reads [`.env.dev`](.env.dev) itself when it runs from the repository root (variables
already set in your environment win), so no shell setup is needed, whichever shell you use.

### 3. Load the example household and create accounts
`config/examples/household.example.toml` is a fictional family with a location, so weather works
right away. Create the database tables first (`serve api` would do it too, but the commands below
need them now), load the household, then give members logins and an adult a PIN:
```bash
cargo run -p fa-server -- migrate
cargo run -p fa-server -- config import config/examples/household.example.toml
cargo run -p fa-server -- user add --member alex --username alex@example.com    # prompts for a password (adults: 12+ chars)
cargo run -p fa-server -- user add --member riley --username riley@example.com  # a child (4+ chars)
cargo run -p fa-server -- pin set --member alex                                  # 4-6 digit PIN for dashboard approvals
```
The import also adds the 8 default weather rules and a disabled example calendar source.
Forgotten a password? `cargo run -p fa-server -- user reset-password --username alex@example.com`.

### 4. Get a weather forecast
The example household is in a fictional place, so the forecast is real weather for those
coordinates. With internet access, fetch it from [Open-Meteo](https://open-meteo.com/) (no key needed):
```bash
cargo run -p fa-server -- weather fetch
```
It prints the current conditions and the advisories the rules produced. **Offline, or to see
advisories on demand?** Use the fixture server in this repo, which serves a forecast that is always
freezing and rainy, plus sample ICS calendars (see the next step):
```bash
node e2e/stubs/server.mjs 18091 &                  # stop it later with: kill %1
FA_OPEN_METEO_URL=http://127.0.0.1:18091 cargo run -p fa-server -- weather fetch
```
(`FA_OPEN_METEO_URL` has to be set for the server too if you want its scheduled fetches, every 30
minutes, to use the stub. Export it in the shell that runs `serve`.)

### 5. Run the app
One process (API and worker together):
```bash
cargo run -p fa-server -- serve all
# with just: just serve   (also builds the web app)
```
or as the **two separate processes** that the compose deployment runs (`serve api` needs the
master key and runs the live-update bridge; `serve worker` runs the job queue, so calendar syncs
and weather evaluation only happen while it is running):
```bash
cargo build -p fa-server
target/debug/fa-server serve api &
target/debug/fa-server serve worker &
# with just: just serve-split
```
Open <http://localhost:8080> and log in as `alex@example.com`.

### 6. Add an ICS calendar
In the app open **Settings, Calendars, Add a calendar source, ICS link**. Enter a name and a link,
press **Test** (it shows the next events) and **Add**. The sample feed from step 4 works:
`http://127.0.0.1:18091/ics/school.ics` (plain `http` is allowed because `.env.dev` sets
`FA_ALLOW_HTTP_FEEDS=true`). The calendar appears under the source, enabled. Use **Sync now** to
fetch it without waiting, and tick the household members it belongs to (none means everyone).
The events show under **Calendar** and on the dashboard.

### 7. See the dashboard
Open **Settings, Devices**, name a device and press **Pair a device**. Open the pairing link
(or scan the QR code) in a browser; it opens `/dashboard?device=...` with the clock, weather,
advisories and the agenda. Without the browser: `cargo run -p fa-server -- device pair --name kitchen`
prints the URL. The page saves the token and removes it from the address bar. *Try elevation*
there exercises the adult-PIN flow. The layout (panels, theme, orientation) is edited per device,
see [The dashboard layout](#the-dashboard-layout).

### 8. Tasks and routines
This step makes a list and a chore, turns it into a task straight away, and checks it off. Log in
as `alex@example.com` and:
1. Open **Settings, Tasks**. On **Lists** add a list named `Chores` (kind *Chores*).
2. On **Chores & schedules** press **Add a chore or schedule**: title `Wipe the counters`, list
   `Chores`, schedule *Every N days after it's done* with 2 days, start date today, strategy
   *Pool* with a few people ticked. Save.
3. The worker creates the task within a minute. To see it immediately, run one pass of the
   scheduler jobs yourself (it uses the same database as the server):
   ```bash
   cargo run -p fa-server -- tasks run-now
   ```
   It prints how many tasks and routine runs it created.
4. Open **Tasks**. The chore is in **Now** and under **Lists**. Tick it: it leaves the open list and
   an **Undo** toast appears (undo works for 10 minutes and restores the schedule).
5. For a **routine**, go to **Settings, Tasks, Routines**, add one (for example `Morning` with three
   steps, *Daily*, starts `00:00`, done by `23:59`, a child ticked) and run `tasks run-now` again.
   The **Routines** page now shows today's run with a progress bar.
6. Pair a device (step 7) and open **Edit layout**: add the **Now**, **Member tasks** and
   **Routine** panels. On the dashboard, completing a task or step asks "Who did it?" first. That
   needs the household's *dashboard completion override*, which is on by default: switch it off
   under **Settings, Household** ("Allow ticking off tasks on dashboards") or with
   `dashboard_completion_override = false` in the `[household]` table of the household TOML
   (omitted: left as it is; new households start with it on). With it off, dashboard panels are
   read-only.

### 9. Phone notifications
`.env.dev` sets `FA_NTFY_URL=http://127.0.0.1:18090`, the dev ntfy server from step 1 (restart
`serve` after changing it). Open **Settings, Notifications**: it shows a *subscribe address*
(`http://127.0.0.1:18090/<topic>`) and a QR code. Without a phone, open that address in a browser:
ntfy's web app shows the topic. Press **Send test** in the app: the test message appears in the
browser within a second. The same from the shell:
```bash
cargo run -p fa-server -- notify test --member alex
```
(it prints the subscribe address; without `FA_NTFY_URL` it says so). Real notifications (a task
assigned to you, overdue, the morning digest) are sent by the worker every minute, and are
grouped and held during quiet hours, see [below](#notifications). On a phone, install the ntfy app
and subscribe to the address on the same network or tailnet.

When you are done, stop the processes (`kill %1 %2 %3` or Ctrl-C) and the containers with
`docker compose -f deploy/compose.dev.yaml stop`.

## Connecting calendars
Open **Settings, Calendars**. A source can be one of four kinds; each source has one or more
calendars, and each calendar can be switched on or off, coloured, and assigned to members.
Events of a calendar with no members are shown for the whole household.

| Kind | How | Notes |
|---|---|---|
| **Google (OAuth)** | **Connect a Google account**, then pick the calendars | Needs a Google OAuth client set up once: [`deploy/README.md`, Google OAuth client](deploy/README.md#google-oauth-client). Hidden in the UI until `FA_GOOGLE_CLIENT_ID` is set. While the Google consent screen is in *Testing* mode the refresh token expires after **7 days** and you must reconnect |
| **Google secret iCal address** | Add it as an **ICS link** | An alternative that needs no OAuth and no 7-day re-consent: in Google Calendar, Settings for the calendar, *Secret address in iCal format*. Google refreshes these slowly |
| **CalDAV** | **CalDAV**, server URL, username and password | Discovers the calendars of the account (Radicale, Nextcloud, iCloud and others). A LAN server over plain `http` needs `FA_ALLOW_HTTP_FEEDS=true` |
| **ICS link** | **ICS link**, then the URL | Any `.ics` URL (school calendars, sports clubs, public holidays) |

Links and passwords are stored encrypted and never shown again; replace them with *Change link or
password*. Sources are polled by the worker (every 15 minutes for Google and CalDAV, hourly for ICS links, which is also a `poll_minutes` setting in the TOML) and **Sync now** forces it.
`config export` never contains secrets, and a source imported from a TOML file starts disabled with
*needs reauth* until you add its link or password in the UI.

**Privacy (D-036):** an adult's private events show as **Busy** on the dashboard and to children.
The household switch is in **Settings, Household**. Children only see events of calendars that
include them or no member.

Known limitation: a saved household location cannot be cleared from the UI.

## Weather and the household location
Set the location, time zone and units in **Settings, Household** (or in the `[household]` table of
the TOML: `tz`, `location = { label, lat, lon }`, `units = "metric" | "imperial"`). Without a
location the app has no forecast. The same table also has `calendar_past_days`,
`calendar_future_days`, `weather_retention_days` and `private_events_as_busy`.

The forecast is fetched from Open-Meteo every 30 minutes by the worker. **Weather rules**
(**Settings, Weather**) turn it into dashboard advisories, for example `min(temp_c, tonight) <= 0`
gives "Frost tonight". The editor checks a condition as you type, shows the error position, and
previews whether it would fire with the latest forecast. Eight default rules are seeded; edit,
disable or add your own.

## Tasks, routines and notifications
Open **Tasks** (everyone) and **Settings, Tasks** (adults). The pieces fit together like this.

### Lists
A **list** groups tasks, for example *Chores*, *Homework* or *Errands*. Its **kind** sets the
defaults, and every one of them can be changed per list:

| Kind | Who may complete | Children may add | On the dashboard "+" | On the calendar |
|---|---|---|---|---|
| Chores | the assignee (or any adult) | no | no | hidden |
| Homework | the assignee | their own | no | until done |
| General | the assignee | no | yes | hidden |

*Adult only* makes a list completable by an adult's own login only. A list can also be hidden from
the dashboard, coloured, and given a **claim time** (how long a claimed pool task stays yours; by
default until the end of the day).

### Chores and schedules
A **chore** (a *task definition*) makes tasks on a schedule. There are two kinds:
- **Every N days after it's done** (`after_done`): the next one is due N days after you finish (or
  skip) the last one. Good for "water the plants": being late never makes the next one pile up.
- **Repeats** (`rrule`): fixed calendar dates, such as every Tuesday or the first of the month. The
  editor builds the rule for you and previews the next dates. It stays on schedule whether or not
  the last one was done.

**Due time** and **lead days** control when a task appears: a task shows up that many days before
it is due. **If it's missed** (the previous task is still open when the next is due) you choose
*Roll over* (keep the open one, create nothing; the default for after-done chores), *Skip it*
(mark the old one skipped and create the new one; the default for repeating chores) or *Pile up*
(create another). A **season** limits a chore to part of the year (a window that wraps the year end
is fine, such as 11-01 to 02-28), and an optional **weather condition** (the same language as
weather rules, for example `sum(precip_mm, today) < 5`) postpones it until the forecast allows.
Missing weather never blocks a chore.

### Who does it
- **Fixed**: one person. If they are away the task stays with them and adults see a flag.
- **Rotation**: take turns in the order you set. By default the turn moves on when anyone
  completes it; you can require that only the person whose turn it is does. Skipping moves the turn
  on unless you turn that off. People who are away on the due date are passed over.
- **Pool**: everyone listed can do it. A member can **claim** it so others see it is taken
  (**Release** gives it back; claims expire).

**Absences** (Settings, Tasks, Absences) record who is away from when to when, and rotation and
pool assignment respect them. Absences are operational data, not part of the TOML export.
Completing, reopening and skipping are written to the audit log with who and from which device.
**Undo** (reopen within 10 minutes of completing) puts the chore's schedule and rotation back as
they were; reopening later only reopens the task.

### Routines
A **routine** is a checklist a member repeats, such as *Morning*: steps with optional target times,
a start time, a "done by" time, and a **day filter**: every day, weekdays, weekends, or **school
days**. School days are weekdays minus days where the chosen *school calendar* has an all-day event
whose title contains one of your keywords ("no school", "holiday", "inset", "closed" by default).
Without a calendar it behaves like weekdays. The worker creates one run per member per qualifying
day; ticking the last step finishes the run, unticking reopens it, and an unfinished run is
skipped the next day. Phones show progress per member (children see their own); the dashboard
**Routine** panel shows large touch targets and a progress bar that turns green at 100%.

### The Now list
**Now** is the short list of what to do next: the top few open tasks (7 by default; **Settings,
Household**, or `now_list_size` in the TOML) by a score that rises as a task approaches its due time and keeps
rising while overdue, plus a bonus for high priority and for tasks made by a weather rule. The
weights are `now_weights` (`due`, `flag`, `weather`) in the TOML. On a phone, **Tasks, Now** lists yours first,
then the household's. The dashboard **Now** panel shows the same list and can be filtered by member
or list in the layout editor.

### Tasks on the calendar
Each task has a calendar display. *Hidden* tasks live only in task views. *Until done* tasks with a
due date show as a chip on the **Calendar** page and the week panel until completed. *Always* tasks
are **key dates** (for example a project due date): they stay after completion, shown as done, and
are the only tasks the agenda panel lists. Homework defaults to *until done*; mark a task as a key
date to make it *always*. Add preparation as steps or as earlier one-off tasks.

### Weather tasks
A **weather rule** (Settings, Weather) with action *Create a task* creates a task from a chore of your
choice once per rule and day when its condition holds ("rain tomorrow: take in the washing"). In
*suggest* mode the task is only proposed: adults see it under **Tasks, Suggested** and **Accept**
or **Dismiss** it.

### Notifications
Notifications use [ntfy](https://ntfy.sh/), a push service you host (the compose file and the dev
setup include one). Each member gets a private random topic; **Settings, Notifications** shows the
subscribe address and a QR code, and **Send test** checks the whole path. Without `FA_NTFY_URL`
notifications are off and the screen says so.

| Trigger | When | On by default |
|---|---|---|
| Assigned | a task is created for you, or someone else adds one for you | yes |
| Overdue | an open task passes its due time (once per task) | yes |
| Digest | a morning summary of today's tasks, when quiet hours end | yes |
| Due soon | a set time before the due time (default 120 minutes) | no |
| Routine nudge | a set time before "done by" while steps remain (default 10 minutes) | no |
| Calendar needs attention | adults only: a calendar source needs to be reconnected | no |

Every member can switch triggers on or off and change the two lead times. **Quiet hours** (a
household default of 21:00 to 07:00, overridable per member) hold messages until they end.
Messages for one member that are at least a minute old are grouped, so a burst of new chores
arrives as "3 updates" with the titles listed. A failed send is retried with growing delays (five
attempts). **Children's copies**: set a child's *copy to* to an adult and that adult receives every
notification of the child, prefixed with the child's name, subject to the adult's own triggers and
quiet hours. Tapping a notification opens the app (the link uses `FA_PUBLIC_BASE_URL`).

### Household TOML
All of this can be written in the household TOML (`fa-server config import`), which checks every
RRULE, weather condition and season condition first and lists all the errors before writing
anything. Topics and absences are never exported. The example below shows only the new keys:
put them into your household file next to its existing `[household]` (which also needs `name`,
`tz` and so on), `[[members]]` and calendar tables. It assumes the members `alex` and `riley` and
the *School calendar* source of `config/examples/household.example.toml`:
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
claim_ttl_minutes = 60                # omitted: until the end of the local day
color = "#44aa88"

[[task_definitions]]
list = "Chores"
title = "Take out the bins"
steps = ["Recycling", "Rubbish"]
rrule = "FREQ=WEEKLY;BYDAY=TU"        # or every_days = 3 (exactly one of the two)
start_date = "2026-01-06"
due_time = "19:00"
lead_days = 1
candidates = ["alex", "riley"]        # member keys, in order
strategy = "rotation"                 # fixed|rotation|pool
season_from = "04-01"
season_until = "10-31"
season_condition = "min(temp_c, today) > 5"

[[routines]]
name = "Morning"
members = ["riley"]
starts_at = "07:00"
done_by = "08:00"
day_filter = "school_days"
school_calendar = "School calendar/School"   # "source name/calendar name"
no_school_match = ["holiday", "inset"]
steps = [{ text = "Brush teeth", target_time = "07:10" }, { text = "Pack bag" }]

[[weather_rules]]
name = "Bins in the rain"
condition = "sum(precip_mm, next 24h) > 5"
action = "task"
severity = "info"
message = "Rain tomorrow"
task_mode = "suggest"                 # create (default)|suggest
task_definition = "Take out the bins" # title, or "List/Title" if ambiguous
```
Import is idempotent by name or title within a list. `weather_rules` may point at a chore
(`task_definition`) from the same file.

## The dashboard layout
Each paired device has its own layout: **Settings, Devices, Edit layout** lets you pick a preset,
orientation, theme, agenda length, and add, remove and reorder panels (clock, weather, advisories,
agenda, week, members, now, member tasks, routine), with a live preview of the device. The layout is stored on the server, so
the screen picks it up on its next refresh.

## Configuration reference
Settings come from environment variables (`FA_*` and `DATABASE_URL`). Local defaults are in
[`.env.dev`](.env.dev); the deployment sample is [`deploy/.env.example`](deploy/.env.example).

| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | none | Postgres URL |
| `FA_MASTER_KEY_PATH` | `/run/secrets/fa_master_key` | 32-byte key file; encrypts stored secrets |
| `FA_BIND_ADDR` | `127.0.0.1:8080` | API listen address (the port the app is served on) |
| `FA_PUBLIC_BASE_URL` | none | URL users reach (cookies, pairing links, the links in notifications) |
| `FA_COOKIE_SECURE` | `true` | `false` only for plain-http local use |
| `FA_WEB_DIST_DIR` | `/srv/web` | The built web app |
| `FA_PUBLIC_URL` | `FA_PUBLIC_BASE_URL` | Base of the Google OAuth redirect (`/api/v1/oauth/google/callback`); https |
| `FA_GOOGLE_CLIENT_ID`, `FA_GOOGLE_CLIENT_SECRET` | unset | Google OAuth client; unset hides Google in the UI |
| `FA_OPEN_METEO_URL` | `https://api.open-meteo.com` | Weather provider (point it at a stub or your own instance) |
| `FA_ALLOW_HTTP_FEEDS` | `false` | Allow plain `http` feeds (LAN CalDAV, local fixtures) |
| `FA_NTFY_URL` | unset | ntfy server that notifications are published to (`http` or `https`; plain `http` is allowed and independent of `FA_ALLOW_HTTP_FEEDS`). Unset (or empty) turns notifications off, and the UI says so. The worker sends every minute |
| `FA_NTFY_TOKEN` | unset | Optional bearer token (a secret) for an ntfy server with access control. Never logged |
| `FA_LOG_LEVEL`, `FA_LOG_FORMAT` | `info`, `pretty` | `trace` to `error`; `pretty` or `json` |

**Ports:** the app serves `8080` (`FA_BIND_ADDR`); the dev database is `55432`; the dev ntfy is `18090`; ntfy in the deployment is `8090`; the e2e stack uses `55499`, `18080`, `18091`, `18092` and `18093` (ntfy).

**`fa-server` commands:**
| Command | What it does |
|---|---|
| `serve [all\|api\|worker]` | Run the server. `api` also runs migrations and the live-update bridge, `worker` runs the jobs, `all` is both in one process |
| `migrate` | Apply migrations and exit |
| `config import <file> [--replace]` / `config export [--out f]` | Household TOML in and out. Import validates RRULEs, weather conditions and season conditions first (naming each one and the position) and fails before writing anything if any is invalid. It seeds the default weather rules. Export never includes secrets or notification topics |
| `tasks run-now` | One pass of `tasks.materialize`, `tasks.tick` and `routines.materialize` in this process: new tasks and routine runs appear at once instead of within a minute |
| `notify test --member <key>` | Publishes a test notification to a member's topic and prints the subscribe address. Fails clearly if `FA_NTFY_URL` is unset |
| `user add --member <key> --username <u>` / `user reset-password` | Accounts (add `--password-stdin` to script it) |
| `pin set --member <key>` | An adult's PIN |
| `device pair --name <n>` / `device revoke <id>` | Dashboard devices |
| `calendar sync <source-id>` | Sync one calendar source now, in this process, and print what changed |
| `weather fetch` | Fetch the forecast now, evaluate the rules and print the result |
| `gen-openapi` | Write the OpenAPI document |

## Day-to-day development
| Task | With `just` | Plain command |
|---|---|---|
| API and worker with auto-reload, plus the Vite dev server with hot reload | `just dev` | `cargo watch -x 'run -p fa-server -- serve all'` and `cd apps/web && corepack pnpm@9 dev` (needs `cargo install cargo-watch`) |
| Regenerate TS types, OpenAPI and the web client after API changes | `just gen` | see the `gen` recipe in [`justfile`](justfile) |
| Tests (Rust and web) | `just test` | `cargo test --workspace` and `cd apps/web && corepack pnpm@9 test` (the Rust tests need `DATABASE_URL`) |
| Lint | `just lint` | `cargo fmt --check && cargo clippy --workspace --all-targets -- -D warnings` |
| Server CLI help | n/a | `cargo run -p fa-server -- --help` |

Vite (`corepack pnpm@9 dev`) serves the UI on its own port and proxies `/api` to `127.0.0.1:8080`.

**Database queries** are checked at compile time by sqlx. After adding or changing a `query!`,
regenerate the offline data with `cargo sqlx prepare --workspace` (install it with
`cargo install sqlx-cli --no-default-features --features rustls,postgres --locked`), and commit
`.sqlx/`. CI compiles with `SQLX_OFFLINE=true`.

**Migrations** live in `crates/fa-store/migrations/` and run automatically when the API starts.
Never edit a migration once it has been deployed anywhere. Add a new one instead.

## Checks: run CI locally
Run these before pushing:
```bash
just ci          # = scripts/ci-local.sh
just e2e         # = scripts/e2e.sh
```
| Script | What it does |
|---|---|
| [`scripts/ci-local.sh`](scripts/ci-local.sh) | Mirrors `.github/workflows/ci.yml`: fmt, clippy, tests against a **fresh throwaway Postgres** with `SQLX_OFFLINE=true`, cargo-deny (if installed), the example-data hygiene check, and web lint, typecheck, test and build. `--no-web` skips the web part |
| [`scripts/e2e.sh`](scripts/e2e.sh) | The end-to-end suite. Starts a throwaway Postgres, `serve api` and `serve worker` as **separate processes**, a fixture server (ICS calendars and an Open-Meteo stub, [`e2e/stubs/`](e2e/stubs/) and [`e2e/fixtures/`](e2e/fixtures/)), Radicale (CalDAV, `tomsquest/docker-radicale`) and a real ntfy (`binwiederhier/ntfy`), seeds the example household, sources, a device, task lists, chores, a routine and a weather rule through the API, runs `tasks run-now`, then runs the Playwright API specs (`e2e/`) and UI specs (`apps/web/e2e/`). Everything is torn down on exit. `FA_E2E_HOLD=<seconds>` keeps the seeded stack up for debugging |
| [`scripts/compose-smoke.sh`](scripts/compose-smoke.sh) | Builds the image, starts `deploy/compose.example.yaml` and takes an on-demand backup |
| [`scripts/restore-rehearsal.sh`](scripts/restore-rehearsal.sh) | Restores a backup dump into a fresh Postgres and checks the app comes up with the same data |
| [`scripts/check-example-data.sh`](scripts/check-example-data.sh) | Fails if example data, fixtures or e2e files contain non-reserved email domains |

The e2e suite stays **local** and is not in CI: it needs Docker, a browser download and a full debug
build, which would not fit the CI time budget (10 minutes). CI runs everything else.

Always test against a **fresh** database. The long-lived dev DB holds state (roles, applied
migrations) that can hide failures CI will hit.

## Deploying
Deployment is up to whoever hosts it (D-033). We ship versioned images (`ghcr.io/<owner>/fa-server`)
and a sample [`deploy/compose.example.yaml`](deploy/compose.example.yaml). The details live in
[`deploy/README.md`](deploy/README.md) and the runbooks in [`deploy/runbooks/`](deploy/runbooks/).
The app is meant to be reachable **only over Tailscale**. Never expose it to the public internet.

**Guidelines**
1. **Pick the layout.** The compose file has profiles: the default profile is Postgres, `api`,
   `worker`, ntfy and nightly backups; `ai-cpu` (small AI models on the core node, from M4) and
   `gpu` (Ollama on a GPU machine) are optional. One machine can run everything; two machines run
   the default profile on the always-on core node and `gpu` on the GPU node
   ([layouts](deploy/README.md#layouts)).
2. **Configure.** `cp deploy/.env.example deploy/.env` and fill in every value: strong
   `POSTGRES_PASSWORD`, `BACKUP_DIR` (your NAS mount), `FA_PUBLIC_BASE_URL`, and optionally the
   feed, Google and ntfy variables (table in [Configuration reference](#configuration-reference)).
   The compose file already runs ntfy and points `FA_NTFY_URL` at it; set `NTFY_BASE_URL` to its
   tailnet address ([ntfy notifications](deploy/README.md#ntfy-notifications)).
   Both `api` and `worker` need outbound HTTPS (calendars, weather, Google).
3. **Create the master key and back it up offline.**
   `head -c 32 /dev/urandom > fa_master_key && chmod 400 fa_master_key`, then keep a copy in a
   password manager or on paper, **separately** from the database backups. It is never in the
   database or the dumps; without it the stored secrets (calendar links and passwords, OAuth
   tokens) cannot be recovered and every source must be reconnected.
4. **Start.** `docker compose --env-file deploy/.env -f deploy/compose.example.yaml up -d`
   (add `--build` to build the image locally). The api runs the migrations on start.
5. **HTTPS on the tailnet.** On the host: `tailscale serve --bg --https=443 http://127.0.0.1:8080`
   (and `--https=8443 ... :8090` for ntfy). Set `FA_PUBLIC_BASE_URL` to that URL.
6. **Google OAuth client (optional).** Follow
   [Google OAuth client](deploy/README.md#google-oauth-client) (human step H-05), set
   `FA_GOOGLE_CLIENT_ID`, `FA_GOOGLE_CLIENT_SECRET` and, if different, `FA_PUBLIC_URL`. Skip it
   to use ICS and CalDAV only.
7. **First run.** Import your household TOML, create accounts, set the location and connect
   calendars, as in [First run](deploy/README.md#first-run).
8. **Pair the Echo Show.** In **Settings, Devices** pair a device and open the pairing link in the
   Echo Show browser (or `fa-server device pair --name echo-show` and open the printed URL). Pick
   and preview its layout under **Edit layout**.
9. **Backups and restore.** The `backup` service writes nightly dumps (7 daily, 4 weekly, 6
   monthly) to `BACKUP_DIR` on the NAS. Rehearse a restore with
   [`scripts/restore-rehearsal.sh`](scripts/restore-rehearsal.sh); the steps are in
   [`deploy/runbooks/restore.md`](deploy/runbooks/restore.md). Rotating the key:
   [`deploy/runbooks/rotate-key.md`](deploy/runbooks/rotate-key.md).
10. **Update.** Pin `FA_VERSION` to a release tag, read the release notes, then
    `docker compose --env-file deploy/.env -f deploy/compose.example.yaml pull && ... up -d`.
    Migrations run when the api starts and stay backward compatible within a minor version.
    Take a backup first (`docker compose ... exec backup /backup.sh`).

Install in one page: [`deploy/runbooks/install.md`](deploy/runbooks/install.md).

## Repository layout
```
crates/          Rust libraries: fa-domain (types and permissions), fa-store (DB, migrations, RLS,
                 encrypted secrets), fa-config, fa-auth, fa-jobs, fa-api, fa-connectors (HTTP clients:
                 Google, CalDAV, ICS, Open-Meteo, ntfy), fa-calendar (sync, recurrence, queries),
                 fa-weather (forecast, rule language, advisories), fa-tasks (schedules, rotation
                 and pools, completion, routines, the Now list), fa-notify (the outbox, quiet
                 hours, batching, ntfy delivery)
bins/            fa-server (server and operator CLI), fa-gpu-agent (GPU node helper, M4)
apps/web/        Vue 3 + TypeScript PWA, including the /dashboard kiosk view
e2e/             Playwright API specs, the fixture server and ICS fixtures (e2e/stubs, e2e/fixtures)
apps/web/e2e/    Playwright UI specs
deploy/          Compose sample, env example, systemd unit, runbooks
scripts/         Dev, CI and ops scripts
config/examples/ Fictional household config
arch/ design/ decisions/ research/ plan/ log/   Design system (see below)
tools/           Python helpers for the design docs (diagram generator, workflow helpers)
```

## How this project is designed
Design work is recorded in the repo, and each folder has an `_index.md`:
- [`PROJECT.md`](PROJECT.md): the original goals and requirements.
- [`arch/`](arch/_index.md): architecture docs, each paired with a generated UML draw.io diagram.
  Start with [`arch/Overall.md`](arch/Overall.md).
- [`design/`](design/_index.md): questions asked during design and their answers.
- [`decisions/`](decisions/_index.md): decision records (D-NNN) that come from those answers.
- [`research/`](research/_index.md): external research the decisions rely on.
- [`plan/`](plan/_index.md): milestone plans and task briefs.
- [`log/`](log/_index.md): a daily work log.

Key principles: it works without AI and AI makes it better; everything is configurable for any
household and hardware; it's local and private; and **no household data ever goes into this
repository**. Examples use `example.com`.

## Roadmap
| Milestone | Scope | Status |
|---|---|---|
| M0 | Skeleton: workspace, DB with RLS, config, auth and RBAC, devices and PIN, jobs, Vue shell, compose, backups, CI | ✅ code-complete |
| M1 | Members, weather, calendar read (Google, CalDAV, ICS), real dashboard | ✅ code-complete ([plan](plan/M1.md)) |
| M2 | Tasks and chores (rotation, pools), routines, the Now list, ntfy notifications | ✅ code-complete ([plan](plan/M2.md)) |
| M3 | Gmail ingest, rules pipeline, bills, school and teacher linking, event approvals | |
| M4 | AI: System One decision models, local LLM, GPU node wake, eval harness | |
| M5 | Recipes, meal plan, grocery list | |
| M6 | Smart assignment, learned calendar routing | |
| M7 | Goals with progress (count or quantity, weekly cadence) | |
| M8 | TUI (stretch) | |

## License
[AGPL-3.0-or-later](LICENSE).
