# Family Assistant

A self-hosted organiser that takes the mental load off a busy household. It pulls together the
family's email, calendars, weather, chores, homework, meals and groceries, and shows what matters
on a wall dashboard and on everyone's phone.

Everything runs locally. Deterministic rules do most of the work, and local AI models help only
where rules can't decide.

> **Status:** M3.6 (themes) is code-complete. M3.5 (onboarding) is code-complete: one compose file, a first-run wizard and optional Tailscale and HTTPS. M3 (mail ingest) is code-complete. On top of the M0 platform (login, members, dashboard
> device pairing with PIN elevation, the job queue, backups, CI), M1 (calendars: Google, CalDAV,
> ICS; the weather forecast with rule-based advisories; a real dashboard) and M2 (task lists,
> recurring chores, routines, the "Now" list, weather tasks, ntfy push notifications) it reads
> email (Gmail API or IMAP), sorts it with deterministic rules, links it to family members, and
> proposes events, bills and tasks that wait for your approval in an inbox. Smart assistance with
> local AI models comes next (see [Roadmap](#roadmap)).

## Contents
- [Run it locally](#run-it-locally)
- [Connecting calendars](#connecting-calendars)
- [Weather and the household location](#weather-and-the-household-location)
- [Tasks, routines and notifications](#tasks-routines-and-notifications)
- [Mail, inbox and bills](#mail-inbox-and-bills)
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

**One-shot path:** `just demo-reset` (or `scripts/demo-reset.sh`) rebuilds the example household, fixtures and a paired device and serves it. It deletes the dev database volume, so it asks you to type `wipe` first.

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

### 3. Create the example household and accounts
`e2e/seed-household.mjs` creates a fictional family (with a location, so weather works right away) through
the first-run setup API and completes the setup, so the browser shows the login page and not the wizard.
It needs the app running, so `scripts/demo-reset.sh` starts the app in the background, seeds it and gives
Riley (a child) a login. To do the same by hand, with the app already running (`cargo run -p fa-server -- serve all`):
```bash
FA_E2E_BASE_URL=http://127.0.0.1:8080 FA_E2E_USER=alex@example.com FA_E2E_PASSWORD='a-long-example-passphrase' \
  FA_E2E_PIN=1234 node e2e/seed-household.mjs        # Alex, the first adult, with a dashboard PIN
cargo run -p fa-server -- user add --member riley --username riley@example.com  # a child (4+ chars); prompts for a password
```
(Node 22.18 or later runs the script directly. To try the wizard instead, start with an empty database and
skip this step.) `.env.dev` sets `FA_ALLOW_PRIVATE_HOSTS=true` so the local stubs below are reachable.
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
   under **Settings, Household** ("Allow ticking off tasks on dashboards"). With it off, dashboard panels are
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

### 10. Mail
To try mail without a real mailbox, start the local [GreenMail](https://greenmail-mail-test.github.io/greenmail/)
server from the dev compose file. It listens only on this machine: IMAP on `127.0.0.1:13143`, SMTP
on `127.0.0.1:13025`, and any login works.
```bash
docker compose -f deploy/compose.dev.yaml up -d greenmail
# with just: just dev-mail
```
The app refuses IMAP without TLS unless you allow it: uncomment `FA_ALLOW_PLAIN_IMAP=true` in
[`.env.dev`](.env.dev) and restart `serve`. (Real mailboxes use TLS; this switch is for this local
server only.) Then, as `alex@example.com`:
1. Open **Settings, Mail**, **Add a mail source**: type *IMAP*, name `Local test`, email address
   `family@example.com`, server `127.0.0.1`, port `13143`, any app password. Press **Test
   connection**: it says "Connected". Leave *Read only* ticked and press **Add source**.
2. Send it a few fictional mails. The ones in [`e2e/fixtures/mail/`](e2e/fixtures/mail/) are written
   for the example household (they come from the school and the biller that
   `e2e/seed-household.mjs` creates):
   ```bash
   for m in teacher-invite newsletter bill permission-slip unknown-teacher; do
     node e2e/send-mail.mjs --port 13025 --to family@example.com --fixture "$m"
   done
   node e2e/send-mail.mjs --port 13025 --to family@example.com --subject "Hello" --body "A plain test mail"
   ```
   (Node 22.18 or later runs the helper directly.)
3. Fetch and sort them now instead of waiting for the five-minute poll:
   ```bash
   cargo run -p fa-server -- mail sync-now
   ```
   It prints how many messages arrived and were processed.
4. Open **Mail** (the list, with filters for category, member and source), **Inbox** (the event
   from the teacher's invite, the suggested task from the permission slip, the new-teacher contact)
   and **Bills** (the utility bill and its "Pay" task). Approving the event needs a calendar that can
   be written to: in **Settings, Calendars** switch on *Allow writing (approved events)* for a CalDAV
   source and mark a calendar as *Default target*.

When you are done, stop the processes (`kill %1 %2 %3` or Ctrl-C) and the containers with
`docker compose -f deploy/compose.dev.yaml stop`.

## Connecting calendars
Open **Settings, Calendars**. A source can be one of four kinds; each source has one or more
calendars, and each calendar can be switched on or off and assigned to members. Events take the colour of their members (several members show several colours).
Events of a calendar with no members are shown for the whole household.

| Kind | How | Notes |
|---|---|---|
| **Google (OAuth)** | **Connect a Google account**, then pick the calendars | Needs a Google OAuth client set up once: [`deploy/README.md`, Google sign-in](deploy/README.md#44-google-sign-in-optional). Hidden in the UI until a Google client ID is set (Settings, System, or `FA_GOOGLE_CLIENT_ID`). While the Google consent screen is in *Testing* mode the refresh token expires after **7 days** and you must reconnect |
| **Google secret iCal address** | Add it as an **ICS link** | An alternative that needs no OAuth and no 7-day re-consent: in Google Calendar, Settings for the calendar, *Secret address in iCal format*. Google refreshes these slowly |
| **CalDAV** | **CalDAV**, server URL, username and password | Discovers the calendars of the account (Radicale, Nextcloud, iCloud and others). A LAN server over plain `http` needs `FA_ALLOW_HTTP_FEEDS=true` |
| **ICS link** | **ICS link**, then the URL | Any `.ics` URL (school calendars, sports clubs, public holidays) |

Links and passwords are stored encrypted and never shown again; replace them with *Change link or
password*. Sources are polled by the worker (every 15 minutes for Google and CalDAV, hourly for ICS links) and **Sync now** forces it.

**Privacy (D-036):** an adult's private events show as **Busy** on the dashboard and to children.
The household switch is in **Settings, Household**. Children only see events of calendars that
include them or no member.

Known limitation: a saved household location cannot be cleared from the UI.

## Weather and the household location
Set the location, time zone and units in **Settings, Household**. Without a
location the app has no forecast. **Settings, Household** also sets the **first day of the week**
(Monday or Sunday, default Monday), which the dashboard's Week view uses.

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
pool assignment respect them.
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
Household**) by a score that rises as a task approaches its due time and keeps
rising while overdue, plus a bonus for high priority and for tasks made by a weather rule. The
weights (`due`, `flag`, `weather`) are in **Settings, Household**. On a phone, **Tasks, Now** lists yours first,
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
quiet hours. Tapping a notification opens the app (the link uses the public address, **Settings, System**).

## Mail, inbox and bills
Mail ingest turns the household's email into a short list of things that need a decision. The
M3 pipeline is **rules only**: deterministic rules, headers, parsers and schema.org data decide,
nothing is sent to a cloud service, and no AI model runs yet (M4 adds those, and what you label now
becomes their evaluation set).

### Sources
**Settings, Mail** adds mailboxes: **Gmail** (Google sign-in with the Gmail API) or **IMAP** (any
provider, an app password). Each source has owners, a visibility (*owners* or the whole
*household*), the number of days of history to read first (30), an optional Gmail search filter, a
poll interval (5 minutes) and a **read only** switch (on by default). Setup, including the Google
scopes and the 7-day re-consent while Google's consent screen is in *Testing* mode, is in
[`deploy/README.md`, Mail](deploy/README.md#43-mail). Sources sync incrementally (Gmail history, IMAP
UIDs), and **Sync now** forces a pass. IMAP passwords and Google tokens are stored encrypted and
never exported.

### What is read, and who sees it
The app stores each message's metadata and, depending on **Settings, Household, Mail and
approvals**, its text (keep it all, keep it for labelling examples only, or purge it after
processing). Adults see the messages of the sources they own, or of *household* sources.
**Children and dashboards never list or read mail**; a dashboard shows at most how many items wait for each adult's approval (a household switch), never what they are.
Message text is never written to the logs.

### Categories and how mail is sorted
Every message gets a **category**, whether it is **time-sensitive**, whether it is a **bill**, and
which **members** it is about. Categories (**Settings, Mail, Categories**) decide what happens
next. The defaults:

| Behaviour | Categories | What it does |
|---|---|---|
| *extract* | school, daycare, activities, appointments, bills, household | looks for events, bills, tasks and contacts; time-sensitive ones notify adults |
| *keep* | personal, newsletters, other | filed and searchable, nothing extracted |
| *ignore* | promotions, social, junk | hidden from the Mail list unless "Show ignored categories" is ticked |

Built-in **signals** (Settings, Mail, Built-in signals) do the sorting: the sender matching an
organization, spam and promotion labels, `List-Unsubscribe` headers, ICS attachments, grade and
class mentions, names and aliases, gender words ("girls"), and so on. Turn off a signal that
guesses wrong for your household. Your own **rules** (Settings, Mail, Rules: sender, domain,
subject or body text, header, organization... then a category, member or flag) run first, can be
tried on past mail before saving, and win over the signals.

### Organizations, contacts and enrollments
**Settings, Organizations** lists schools, daycares, clubs and billers: their domains (subdomains
match) and sender addresses, an optional ICS feed and default calendar, a **contact mode**
(*suggest* new senders at the domain for confirmation, add them *automatically*, or *manual*), and
for billers a **bill template** (regular expressions for the amount, the due date and the account).
Contacts are the teachers, coaches and offices. A child's **enrollment** (on **Settings, Organizations**) records the organization, grade, class and the contacts
there, which is how "grade 3 families" mail from the school reaches the right child. Members (on **Members**) also
have aliases (nicknames), a gender and relevance tags (such as `soccer`) that help the linking.

### The inbox: why everything waits for approval
Nothing extracted is trusted blindly in M3: **every extracted event becomes a proposal**, as do
bills that are not clearly bills, tasks the mail suggests and new contacts. **Inbox** lists them
under Events, Bills, Tasks and Contacts, each editable before you approve (change the title, time,
calendar or assignees). **Approve** an event and it is written to the chosen calendar (Google or
CalDAV), once: a later sync of that calendar does not duplicate it, and a repeat mention in another
mail only adds provenance. A cancellation withdraws a pending proposal, or proposes removing the
event it wrote. Proposals that are not decided expire (events when they start; tasks and
contacts after 14 days). Adults get an **approval needed** notification, sent immediately, batched
(the default) or only in the morning digest, or switched off, per member under **Settings,
Notifications**. Approvals happen only in the app's **Inbox**, and each adult sees only the items from sources they may
see. The wall **dashboard** has no inbox panel, only an optional per-adult pending count (**Settings,
Household**). **Settings, Household, Mail and
approvals** holds the thresholds, the expiry days and an automatic-write switch for
high-confidence events (off by default).

### Bills and pay tasks
A bill is recognised from an organization's template or from generic patterns (and PDF text). A
clear one (confidence 0.8 or more) becomes an open **bill** and a **Pay <payee>** task on the
*Bills* list for any adult, due a few days before the due date (`bill_lead_days`, 3). Completing the
task marks the bill **paid**; reopening the task reopens the bill. **Bills** shows them by status.
Money is kept in minor units with a currency (`currency` in the household, `USD` by default).

### Actions and undo
On a **writable** source, each category's *automatic actions* run: by default a label
`FA/<category>` and `FA/<member>` (real Gmail labels, or IMAP keywords); *archive* and *mark read*
are optional per category. A message page also has manual actions and **Make task**. Every
action can be **undone for 30 days**; a read-only source refuses them.

### Labelling
**Mail, Label mail** serves messages one by one (balanced across categories) with the four
decisions as chips: category, time-sensitive, who it is about and is-it-a-bill. **Looks right**
records your confirmation, and fixing a chip records your answer as a human decision. Tick *In the
evaluation sample* to count a message toward the held-out set; the progress bar counts labelled
messages of that sample (200 by default).

## The dashboard layout
Each paired device has its own layout: **Settings, Devices, Edit layout** lets you pick a preset,
orientation, theme, agenda length, and add, remove and reorder panels (clock, weather (which also shows advisories and upcoming thunderstorms),
agenda, week, members, now, member tasks, routine), with a live preview of the device. The layout is stored on the server, so
the screen picks it up on its next refresh.

The dashboard has a navigation bar on its shorter edge (left in landscape, top in portrait) with two
views: **Home** (your layout) and **Week** (a table with a row per day and a column per person, for the week starting on the household's first day of the week, with arrows to move between weeks and a switch to hide events that have passed; set the first day in **Settings, Household**). The view scrolls when its content is taller than the screen.

## Themes
**Using it.** Open **Settings, Appearance** (any signed-in member) to pick one of eight palettes:
Catppuccin, Nord, Dracula, Gruvbox, Solarized, Tokyo Night, Rosé Pine, Kanagawa and seoul256 (Nord has a light variant too). The choice
applies at once and is saved **per device** in the browser (`localStorage`, key `fa.theme`); it does
not follow you to another device, and there is no household default (Catppuccin, automatic
light/dark, is what a device uses until a choice is made). A family with both light and dark
variants follows the device's light/dark setting in `auto`, or the mode you pick; a family with one
variant (Nord) always uses it. The kiosk's theme is set by an adult in **Settings, Devices, Edit
layout** (family, variant and mode; `auto` is dark between sunset and sunrise). A member's colour
is one of eight hue names (red, orange, yellow, green, cyan, blue, purple, pink), drawn in the
active theme's shade.

**Adding a family** (contributors). Add `apps/web/src/themes/families/<id>.ts` exporting a
`ThemeFamily` (`id` matching `^[a-z0-9][a-z0-9-]{0,31}$`, `label`, `source` URL, `license`, and
`variants`; each variant has `neutrals` and all eight `hues` as `#rrggbb`, from the project's
published palette; mark anything derived in a comment), then list it in `FAMILIES` in
`apps/web/src/themes/families/index.ts`. The contrast test checks every variant (text and muted at
4.5:1 on bg and surfaces, each hue at 3:1 on bg, on-accent on blue at 4.5:1):
`cd apps/web && corepack pnpm@9 test src/themes`. Record the source and licence in
`research/R-002-theme-palettes.md`.

## Configuration reference
Settings come from environment variables (`FA_*` and `DATABASE_URL`), and an optional `fa.toml` (process settings only, never household data)
(path in `FA_CONFIG`; a key set there counts as an environment value). Local defaults are in
[`.env.dev`](.env.dev); the deployment's compose variables are in [`deploy/.env.example`](deploy/.env.example)
(all optional).

### Process settings (environment only)
| Variable | Default | Meaning |
|---|---|---|
| `DATABASE_URL` | none | Postgres URL |
| `FA_DATABASE_PASSWORD_FILE` | unset | File holding the DB password, inserted into `DATABASE_URL` (which then has a user name and no password). The compose file uses it |
| `FA_MASTER_KEY_PATH` | `/run/secrets/fa_master_key` | 32-byte key file; encrypts stored secrets. The compose file points it at `/run/fa-secrets/master_key`, made by `fa-server init` |
| `FA_BIND_ADDR` | `127.0.0.1:8080` | API listen address (compose: `0.0.0.0:8080`) |
| `FA_COOKIE_SECURE` | `auto` | `auto` marks the session cookie `Secure` when the request came over TLS, or over a trusted proxy with `X-Forwarded-Proto: https`; `true` or `false` force it. HSTS is never sent |
| `FA_WEB_DIST_DIR` | `/srv/web` | The built web app |
| `FA_BACKUP_DIR` | unset | Read-only view of the backups, for the app's own checks |
| `FA_LAN_DASHBOARD_BIND` | `0.0.0.0:8081` | Where the home-network dashboard listener binds (inside the container) once the setting below is on |
| `FA_PUBLIC_URL` | `tailnet_base_url`, else the effective public address | Base of the Google OAuth redirect (`/api/v1/oauth/google/callback`) and of links produced without a request; Google needs `https` |
| `FA_OPEN_METEO_URL` | `https://api.open-meteo.com` | Weather provider (point it at a stub or your own instance) |
| `FA_GOOGLE_API_BASE` | `https://www.googleapis.com` | Base URL of the Gmail and Google Calendar calls. For stubs and tests, not for production |
| `FA_ALLOW_PLAIN_IMAP` | `false` | Allow IMAP without TLS. **Tests, e2e and the local GreenMail only**; the compose file does not pass it through |
| `FA_IMAP_TLS_INSECURE` | `false` | Accept self-signed IMAPS certificates. **The GreenMail tests only** |
| `FA_SECRETS_DIR` | `/run/fa-secrets` | Where `fa-server init` writes the master key and database password |
| `FA_LOG_LEVEL`, `FA_LOG_FORMAT` | `info`, `pretty` (compose: `json`) | `trace` to `error`; `pretty` or `json` |

### Settings you can also change in the app (Settings, System)
Precedence, highest first (D-075): **an environment variable (or a key in `fa.toml`)**, then the value saved
in **Settings, System** (table `instance_setting`; secrets are stored encrypted), then **`FA_DEFAULT_<KEY>`**,
then the built-in default. A setting given as a plain environment variable is **locked**: the UI shows it
greyed out. `FA_DEFAULT_<KEY>` sets a starting value without locking (the compose file uses it for the ntfy
address and the trusted proxies). An **empty** variable counts as unset: it neither locks nor hides the saved
value. Saving a changed setting restarts the app by itself in about a second, only if the effective values changed.

| Setting (UI label) | Variable (`FA_DEFAULT_` + the key in capitals) | Default | Meaning |
|---|---|---|---|
| Public address (`public_base_url`) | `FA_PUBLIC_BASE_URL` | the wizard saves the browser's address | URL users reach: pairing links, the links in notifications. Optional |
| Tailscale address (`tailnet_base_url`) | `FA_TAILNET_BASE_URL` | unset | The HTTPS tailnet address; used for the Google redirect and as a notification link base |
| Notifications server (`ntfy_url`) | `FA_NTFY_URL` | compose: `http://ntfy:80` | ntfy server the app publishes to (`http` or `https`; independent of the feeds switch). Unset turns notifications off |
| Notifications address for phones (`ntfy_public_url`) | `FA_NTFY_PUBLIC_URL` | empty: the tailnet address on `:8443`, else this server on `:8090`, else the server above | The ntfy address in subscribe links and QR codes |
| Notifications token (`ntfy_token`) | `FA_NTFY_TOKEN` | unset | Bearer token (secret) for an ntfy server with access control. Never logged |
| Google client ID, secret (`google_client_id`, `google_client_secret`) | `FA_GOOGLE_CLIENT_ID`, `FA_GOOGLE_CLIENT_SECRET` | unset | Google OAuth client; without it Google is hidden. Needs an `https` redirect, so HTTPS access |
| Allow plain-HTTP feeds (`allow_http_feeds`) | `FA_ALLOW_HTTP_FEEDS` | `false` | Accept plain `http` for feeds and OAuth redirects (LAN CalDAV, dev) |
| Allow feeds on private addresses (`allow_private_hosts`) | `FA_ALLOW_PRIVATE_HOSTS` | `false` | Let calendars, feeds and the like reach private and loopback addresses (SSRF guard, C-05). Turn it on for a LAN CalDAV server, or in dev for local stubs (`.env.dev` does). IMAP is not affected |
| Trusted proxies (`trusted_proxies`) | `FA_TRUSTED_PROXIES` | loopback only; compose: loopback and the Tailscale sidecar `172.30.89.10/32` | Comma-separated CIDRs whose `X-Forwarded-*` headers are trusted (client IP for login limiting, `https` detection for the cookie). Add your own reverse proxy here |
| Home-network dashboard address (`lan_dashboard_url`) | `FA_LAN_DASHBOARD_URL` | unset | The dashboard's address on the home network, such as `http://192.168.1.10:8081`. Setting it turns on a **dashboard-only listener** for screens that can't use Tailscale (D-064): device tokens only, only the dashboard's own API. You also publish the port yourself |
| Allowed networks (`lan_allowed_cidrs`) | `FA_LAN_ALLOWED_CIDRS` | private and loopback ranges | CIDRs allowed to use that listener, by peer address (`X-Forwarded-For` is ignored there) |
| Allow PIN elevation (`lan_allow_pin`) | `FA_LAN_ALLOW_PIN` | `false` | Allow PIN elevation on that listener. It is plain HTTP, so a PIN would cross the LAN unencrypted |

### Compose variables (`deploy/.env`, all optional)
`FA_HTTP_HOST_IP` (`0.0.0.0`), `FA_HTTP_PORT` (`8080`), `FA_NTFY_PORT` (`8090`), `FA_VERSION` (`latest`),
`BACKUP_DIR` (the volume `fa-backups`), `TS_AUTHKEY`, `TS_HOSTNAME` (`family`), `FA_LOG_LEVEL`, `NTFY_BASE_URL`,
`NTFY_UPSTREAM_BASE_URL` and `OLLAMA_BIND`. They are described in [`deploy/.env.example`](deploy/.env.example).

**Ports:** the app serves `8080` (`FA_BIND_ADDR`); the dev database is `55432`; the dev ntfy is `18090`; the dev GreenMail is `13143` (IMAP) and `13025` (SMTP); the deployment publishes the app on `8080` (`FA_HTTP_PORT`) and ntfy on `8090` (`FA_NTFY_PORT`), and the optional home-network dashboard listener is `8081` inside the container (you publish it yourself, see the [guide](deploy/README.md#45-the-wall-screen-dashboard)); the e2e stack uses `55499`, `18080`, `18081` (home-network dashboard), `18091`, `18092`, `18093` (ntfy), `18143` (GreenMail IMAP), `18025` (GreenMail SMTP), and `18094`/`18095` for the two empty-database instances (setup API and the plain-HTTP wizard; `FA_E2E_SETUP_PORT`, `FA_E2E_WIZARD_PORT`); `scripts/ci-local.sh` uses `55496`, `55497` and `55498`; `scripts/compose-smoke.sh` uses `18180`, `18182` and `18190`.

**`fa-server` commands:**
| Command | What it does |
|---|---|
| `serve [all\|api\|worker]` | Run the server. `api` also runs migrations and the live-update bridge, `worker` runs the jobs, `all` is both in one process (what compose runs) |
| `init [--secrets-dir <dir>]` | Create the master key and the database password in the secrets folder if absent, never overwriting. Runs before any config loading; compose runs it as the one-shot `init` service |
| `migrate` | Apply migrations and exit |
| `tasks run-now` | One pass of `tasks.materialize`, `tasks.tick` and `routines.materialize` in this process: new tasks and routine runs appear at once instead of within a minute |
| `notify test --member <name>` | Publishes a test notification to a member's topic and prints the subscribe address. Fails clearly if no ntfy server is set |
| `user add --member <name> --username <u>` / `user reset-password` | Accounts (add `--password-stdin` to script it). `<name>` is the member's display name. A recovery and scripting tool: the wizard and Settings, Members cover everyday use |
| `pin set --member <name>` | An adult's PIN (recovery and scripting, as above) |
| `device pair --name <n>` / `device revoke <id>` | Dashboard devices. Pairing uses the saved settings and prints the home-network link too when that address is set |
| `mail sync-now [--source <name>]` | One sync pass of every mail source (or the named one), then processes the new messages, in this process. Needs `FA_ALLOW_PLAIN_IMAP` for a plain-IMAP server |
| `mail process-now` | Process the household's pending messages in this process |
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
| IMAP integration tests (a real GreenMail) | n/a | see below |

Vite (`corepack pnpm@9 dev`) serves the UI on its own port and proxies `/api` to `127.0.0.1:8080`.

**IMAP integration tests** (`fa-connectors`, `fa-mail`) need a throwaway GreenMail and are ignored
unless `FA_TEST_GREENMAIL=host:imap_port:imaps_port` is set. `scripts/ci-local.sh` and CI run them.
By hand:
```bash
docker run -d --name fa-t02-greenmail -p 33143:3143 -p 33993:3993 \
  -e GREENMAIL_OPTS='-Dgreenmail.setup.test.imap -Dgreenmail.setup.test.imaps \
  -Dgreenmail.hostname=0.0.0.0 -Dgreenmail.auth.disabled' greenmail/standalone:2.1.8
sleep 10   # GreenMail needs a few seconds before it answers
FA_TEST_GREENMAIL=127.0.0.1:33143:33993 cargo test -p fa-connectors --test imap -- --include-ignored
FA_TEST_GREENMAIL=127.0.0.1:33143:33993 cargo test -p fa-mail --test sync_imap -- --include-ignored
docker rm -f -v fa-t02-greenmail
```
(`fa-mail`'s tests also need `DATABASE_URL`.)

**Database queries** are checked at compile time by sqlx. After adding or changing a `query!`,
regenerate the offline data with `cargo sqlx prepare --workspace` (install it with
`cargo install sqlx-cli --no-default-features --features rustls,postgres --locked`), and commit
`.sqlx/`. Builds use that offline data by default (`SQLX_OFFLINE=true` in `.cargo/config.toml`),
so compiling never needs a running or migrated database. To check queries against a live database
instead, run with `SQLX_OFFLINE=false`.

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
| [`IMPROVEMENTS.md`](IMPROVEMENTS.md) | Outstanding review findings and suggestions, by area |
| `python3 tools/fa.py check` | Lints the design tree (`arch/`, `design/`, `decisions/`, `log/`, `plan/`, `research/`): front-matter keys, status values, id/filename parity, duplicate numbers, index ⇄ file parity. CI runs it |
| [`scripts/ci-local.sh`](scripts/ci-local.sh) | Mirrors `.github/workflows/ci.yml`: fmt, clippy, tests against a **fresh throwaway Postgres** with `SQLX_OFFLINE=true`, the GreenMail-gated IMAP tests (a throwaway GreenMail), cargo-deny (if installed), the example-data hygiene check, and web lint, typecheck, test and build. `--no-web` skips the web part |
| [`scripts/e2e.sh`](scripts/e2e.sh) | The end-to-end suite. Starts a throwaway Postgres, `serve api` and `serve worker` as **separate processes**, a fixture server (ICS calendars and an Open-Meteo stub, [`e2e/stubs/`](e2e/stubs/) and [`e2e/fixtures/`](e2e/fixtures/)), Radicale (CalDAV, `tomsquest/docker-radicale`, also the writable calendar for approved events), a real ntfy (`binwiederhier/ntfy`) and a GreenMail (`greenmail/standalone`, fictional fixture mails from [`e2e/fixtures/mail/`](e2e/fixtures/mail/)), seeds the example household, sources, a device, task lists, chores, a routine, a weather rule and an IMAP mailbox through the API, runs `tasks run-now` and `mail sync-now`, then runs the Playwright API specs (`e2e/`) and UI specs (`apps/web/e2e/`). Everything is torn down on exit. It also starts two empty-database instances for the setup API and wizard specs; the wizard one is browsed through the host's LAN address so it is a plain-HTTP, non-secure context. `FA_E2E_HOLD=<seconds>` keeps the seeded stack up for debugging |
| [`scripts/compose-smoke.sh`](scripts/compose-smoke.sh) | Builds the image, starts `deploy/compose.yaml` (with the build overlay) with **no `.env`**, checks the generated secrets and their idempotence, runs setup through the API over plain HTTP (no `Secure` cookie), takes an on-demand backup and checks that `down -v` leaves no volumes. `FA_SMOKE_IMAGE` (optional) tests a prebuilt image instead. CI runs it as the `compose-smoke` job |
| [`scripts/restore-rehearsal.sh`](scripts/restore-rehearsal.sh) | Restores a backup dump into a fresh Postgres and checks the app comes up with the same data |
| [`scripts/check-example-data.sh`](scripts/check-example-data.sh) | Fails if example data, fixtures or e2e files contain email domains other than `example.com`, `example.org`, `example.net`, `example.edu` (the fictional school) and `.test`/`.invalid` |

The e2e suite stays **local** and is not in CI: it needs Docker, a browser download and a full debug
build, which would not fit the CI time budget (10 minutes). CI runs everything else.

Always test against a **fresh** database. The long-lived dev DB holds state (roles, applied
migrations) that can hide failures CI will hit.

## Deploying
**Follow the step-by-step guide: [Install on a home server](deploy/README.md).** It assumes no prior
Docker knowledge, and ends with the app on your phones, with calendars, mail and notifications connected.

In short, on a machine with Docker (Compose 2.23.1 or newer):

```bash
curl -fsSLO https://raw.githubusercontent.com/JonGanz/family-assistant/<release-tag>/deploy/compose.yaml
docker compose up -d
```
then open `http://<server>:8080` and **finish the setup wizard right away**: nothing else protects the first
visit. No `.env` or key files are needed: the one-shot `init` service creates the master key and
the database password in the `fa-secrets` volume, and the wizard creates the household, the logins, the public address, the
notification settings and shows the master key to back up. Until the first release image is published
(human step H-12), build locally: `GIT_COMMIT=$(git rev-parse --short=12 HEAD) docker compose -f deploy/compose.yaml -f deploy/compose.build.yaml up -d --build` (the variable is optional; it is the commit that Settings, System, About shows).

| Want | Do |
|---|---|
| Remote access over HTTPS | `TS_AUTHKEY=... docker compose --profile tailscale up -d`: a Tailscale sidecar serves `https://<name>.<tailnet>.ts.net` (and ntfy on `:8443`) |
| Plain HTTP at home | Nothing: the app works over `http://<server>:8080`; the cookie is `Secure` only for HTTPS requests |
| Your own reverse proxy | Add its address under **Settings, System, Trusted proxies** |
| A different port, version or backup folder | Copy [`deploy/.env.example`](deploy/.env.example) to `.env` (all optional) |
| A wall screen without Tailscale | The opt-in dashboard-only listener (D-064), see the [guide](deploy/README.md#45-the-wall-screen-dashboard) |
| Backups | Nightly, into the `fa-backups` volume or `BACKUP_DIR`; the master key is **not** in them, keep the wizard's copy |

**Never expose the app to the public internet.** The default services are `init`, `postgres`, `app` (the API,
the web app and the jobs in one container), `ntfy` and `backup`; profiles add `tailscale`, `gpu` (Ollama) and
`ai-cpu` (a placeholder until M4). Settings that belong to the installation live in **Settings, System** (see the
[Configuration reference](#configuration-reference)). The runbooks for restoring a backup and rotating the key
are in [`deploy/runbooks/`](deploy/runbooks/).

## Repository layout
```
crates/          Rust libraries: fa-domain (types and permissions), fa-store (DB, migrations, RLS,
                 encrypted secrets), fa-config, fa-auth, fa-jobs, fa-api, fa-connectors (HTTP clients:
                 Google, CalDAV, ICS, Open-Meteo, ntfy), fa-calendar (sync, recurrence, queries),
                 fa-weather (forecast, rule language, advisories), fa-tasks (schedules, rotation
                 and pools, completion, routines, the Now list), fa-notify (the outbox, quiet
                 hours, batching, ntfy delivery), fa-rules (pure mail rules: parsing, signals,
                 extractors), fa-pipeline (the decision pipeline engine and its decision log),
                 fa-mail (sources, Gmail and IMAP sync, processing, the inbox, bills and
                 actions, labelling)
bins/            fa-server (server and operator CLI), fa-gpu-agent (GPU node helper, M4)
apps/web/        Vue 3 + TypeScript PWA, including the /dashboard kiosk view
e2e/             Playwright API specs, the fixture server, ICS and mail fixtures (e2e/stubs, e2e/fixtures),
                 the mail sender (e2e/mail-send.ts, e2e/send-mail.mjs)
apps/web/e2e/    Playwright UI specs
deploy/          compose.yaml (install file), compose.build.yaml (build overlay), compose.dev.yaml, env example,
                 systemd unit, guide and runbooks
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
| M3 | Email ingest (Gmail API or IMAP), rules pipeline, bills, school and teacher linking, event approvals | ✅ code-complete ([plan](plan/M3.md)) |
| M3.5 | Onboarding (one compose file, optional Tailscale sidecar, optional HTTPS, first-run wizard) and a real-hardware shakedown (D-065, D-066) | ✅ code-complete; the shakedown on the real server (H-12 to H-14, then H-02 to H-11) is pending ([plan](plan/M3.5.md), [human steps](plan/human-steps.md)) |
| M3.6 | Themes: eight palette families, a per-device choice, a named hue per member, the kiosk on the same catalogue (D-079 to D-081) | code-complete; integration (CI, e2e, compose smoke) is pending ([plan](plan/M3.6.md)) |
| M4 | AI, minimal slice: one System One model and one local LLM behind the pipeline (D-067) | |
| M4.1 | AI platform: backend registry, hardware probe, GPU node wake and gaming detection, eval harness | |
| M5 | Recipes, meal plan, grocery list | |
| M6 | Smart assignment, learned calendar routing | |
| M7 | Goals with progress (count or quantity, weekly cadence) | |
| M8 | TUI (stretch) | |

## License
[AGPL-3.0-or-later](LICENSE).
