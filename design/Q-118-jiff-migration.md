---
id: Q-118
title: Move dates and time zones from chrono to jiff
status: open
batch: 26
area: tech
created: 2026-11-18
resolved:
decisions: []
---

# Q-118: Move dates and time zones from chrono to jiff

## Context
RUSTSEC-2026-0543 and RUSTSEC-2026-0555 (2026-11-17) mark chrono and chrono-tz unmaintained; they are ignored in `deny.toml` for now. A recent real-world rule change showed why stale rules matter: every event, routine day and "today" is computed in the household's zone. chrono is in 180 files and behind sqlx, utoipa and ts-rs; chrono-tz is in 11 crates. jiff 1.x has no native sqlx, utoipa or ts-rs support yet (utoipa supports only jiff 0.2), so a full move needs wrapper types and generator overrides (R-008). The web app's own times come from the device's browser data and are not affected.

## Question
chrono and chrono-tz are now unmaintained (R-008). chrono-tz bakes the time-zone rules into the binary and will stop following rule changes. How far should we move to jiff?

## Options
- A. Time zones first (recommended): replace chrono-tz with jiff's time-zone database now, keep chrono types for dates and timestamps and convert at the edges; move the rest later in its own milestone
- B. Full move in one milestone: jiff types everywhere, jiff-sqlx for the database, overrides for the OpenAPI and TypeScript generators; chrono stays only inside third-party crates
- C. Stay on chrono and chrono-tz for now, rebuild whenever chrono-tz publishes, revisit later

## Answer
<!-- answer below this line -->

