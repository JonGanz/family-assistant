# Changelog

Every tagged release adds its entry here, in the same commit. Newest first.

## Unreleased

- Nothing yet.

## v0.1.32 — 2026-11-14

- Release notes on System > About.
- Recipes: choosing a new image and saving now uploads it (it silently did nothing before); phone photos are shrunk before upload and upload errors are shown.
- Meal Plan is an agenda that starts two days ago, instead of weeks.
- Today shows who each task is assigned to; the Tasks page shows members as round avatars.
- Week on phones: square event corners and events at the top of each day.
- Dashboard recipe: the details card sits beside the image on wall screens.

## v0.1.31 — 2026-11-13

- Month view: member marks are square and the text is larger on phones.
- Week view: a per-device switch hides past days; phone cells are tidier with equal row heights.
- Dashboard: each routine gets its own card, with confetti bursts and a rising stream when you finish one.
- Recipes: a view with a details card and a checkbox per recipe.

## v0.1.30 — 2026-11-12

- Calendar no longer has a top bar; Month fills the screen with today inverted.
- Week fits on a phone and Agenda is capped at a readable width.
- Dashboard snow now appears only when the weather forecast has snow, and snow showers fall faster and denser.

## v0.1.29 — 2026-11-12

- Month is tighter and today is marked.
- Week fits on mobile and Agenda opens on today.
- Dashboard confetti and snow effects, each with a per-device switch.

## v0.1.28 — 2026-11-12

- The kiosk scales to fit any screen size.
- Week bar restyled, with hide-past turned on by default; tabs show at every width.
- Month is tighter.

## v0.1.27 — 2026-11-12

- Kiosk portrait mode: routines stay on screen and scroll inside their card.

## v0.1.26 — 2026-11-11

- Kiosk PIN pad redesigned.
- Agenda scrolls under fixed tabs and filter.
- Larger kiosk meal plan with links to recipes.
- Kiosk portrait: two routines side by side, pinned to the bottom and never off screen.

## v0.1.25 — 2026-11-11

- Inbox: Discard button, and an AI duplicate check for proposed events.
- Dashboard Timers.
- Calendar Month fits the screen with one-line items, no tasks and +N for overflow; Week has equal rows and a +N events day list; Agenda scrolls endlessly from today.
- Kiosk: lock, fullscreen and status (only when unhealthy) sit on the right; a PIN popover appears at the lock; wider undo toast.
- Dashboard Home shows meals in one card; Today weather has a scrolling hourly breakdown.

## v0.1.24 — 2026-11-10

- Job errors show on the Status page with copy and retry.
- Meal cards per slot, a Tomorrow forecast strip, and calendar tabs listed first.
- Centred dashboard task checkboxes and icon menu buttons.

## v0.1.23 — 2026-11-10

- Today events show member chips and trims again.
- One trim style for alerts, notices and cards.

## v0.1.22 — 2026-11-10

- Rounded member pills on dashboard events and a restaurant food icon.
- Shared tabs for Food, Calendar, Inbox and Tasks.

## v0.1.21 — 2026-11-10

- Today page redesigned, with Tomorrow, a day sheet and map links.
- Calendar chevrons for moving between periods.
- AI-only mail mode, and a night-time device theme.

## v0.1.20 — 2026-11-10

- Home spacing and portrait routines improved.
- Meal slots open a recipe window; dashboard windows are rounded.
- Icon navigation, locked task checkboxes and chips, and uniform event trims.

## v0.1.19 — 2026-11-09

- Dashboard cards, meals on Home, and rounded multi-person events.
- Calendar week grid, an AI Process now button, and a clock screensaver.
- The kiosk Food page shows a rolling 7-day meal plan.

## v0.1.18 — 2026-11-09

- AI endpoint thinking switch (off by default) and larger output budgets for the AI.

## v0.1.17 — 2026-11-08

- Operator command to start mail processing fresh.

## v0.1.16 — 2026-11-08

- Open apps and the kiosk pick up a new version after an update.

## v0.1.15 — 2026-11-08

- Kiosk meal pages work on the home network listener.

## v0.1.14 — 2026-11-08

- Meals: plan, templates, shopping list that works offline, recipes (read from web pages) and meal settings.
- Kiosk Food view and a meals panel.
- Ingredient normalisation and background meal jobs.

## v0.1.13 — 2026-11-07

- New look across every page.
- Mail is read by the AI first, with rules as a fallback and a catch-up queue; AI mode, thresholds and backlog live in Settings.
- Inbox task suggestions link to their source email.
- Phones get a menu drawer; desktops get a collapsible one.
- Calendar remembers its view; filter chips add up; new editor-style theme palettes.

## v0.1.12 — 2026-11-03

- Portrait dashboard stacks routines vertically.
- The completed-routine message is gone.
- Security update to a bundled library.

## v0.1.11 — 2026-11-02

- Hardening of lists, mail, calendar and notifications: size limits, timeouts, safer retries and fewer race conditions.
- Stored secrets are bound to their owner.
- Lists load more on request; refused task completions lock only that task and undo works better.

## v0.1.10 — 2026-11-01

- Dashboard: fullscreen button and a non-scrolling kiosk page.
- A refused completion locks only that task; adults-only tasks have no checkbox.
- Updated dependencies.

## v0.1.9 — 2026-10-31

- Whiteboard: separate thickness selector and a shape tool (rectangle, circle, triangle, hexagon).
- Week view adapts its slots to show two rows in landscape and three in portrait.

## v0.1.8 — 2026-10-31

- Week view: bigger event titles and adaptive slot height.
- Agenda has more space between days; bigger gap on the portrait home.

## v0.1.7 — 2026-10-31

- Dashboard whiteboard with pen, eraser, theme colours and local storage.
- Round colour swatches with a glow, clear confirmation, and visible ink in dark mode.

## v0.1.6 — 2026-10-30

- Week view inverts today's date cell.
- Portrait agenda rows with more spacing, and clearer forecast columns.

## v0.1.5 — 2026-10-30

- Portrait home: top band, members strip, and scrolling Now and Coming up columns.
- Landscape home with scrolling columns; improved windows and dashboard styles.

## v0.1.4 — 2026-10-30

- Calendar sample data for trying the app.
- Under the hood: image build fix.

## v0.1.3 — 2026-10-30

- A screensaver.
- Bigger dates in Week view, auto orientation and a fixed menu position.
- Simpler configuration (TOML options removed).

## v0.1.2 — 2026-10-30

- Under the hood: release build fix.

## v0.1.1 — 2026-10-30

- About page, theme support and a Dockerfile build speed-up.
- Dashboard weekly view, visual and square-corner refinements.
- Adult members can be added without errors; adults no longer have a grade field.
- Calendar fixes: sync no longer fails on an empty reply and bad repeat end dates only warn.
- Clearer master key setup page.

## v0.1.0 — 2026-10-27

- First tagged release: household, members, calendar, tasks and the dashboard, with onboarding and a deployment guide for a home server.
- Rate-limited PIN elevation, live updates that resync after a gap, and a unified look across pages.
