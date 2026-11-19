---
id: R-008
title: Moving from chrono and chrono-tz to jiff
date: 2026-11-18
method: Main session. Read the RustSec advisories RUSTSEC-2026-0543 and RUSTSEC-2026-0555 and the announcement chronotope/chrono#1768 (fetched 2026-11-18). Crate versions and features from `cargo search` and `cargo info` and from the Cargo.toml files in the local cargo registry. The runtime image's time-zone data was checked with `docker run --rm debian:bookworm-slim`. Usage counts are from grep over crates/ and bins/.
verified: Verified = advisory category and dates; the maintainer's statement that chrono-tz will stop tracking time-zone database updates; jiff 1.0.1 default features (tz-system, tzdb-zoneinfo, tzdb-bundle-platform); jiff-sqlx 1.0.0 (postgres feature); utoipa 5.5.0 supports jiff_0_2 only; ts-rs 11.1.0 has chrono-impl and no jiff feature; debian:bookworm-slim ships tzdata 2026c; 180 .rs files use chrono and 11 crates use chrono-tz directly. NOT verified = how jiff picks between the system database and its bundled copy at runtime beyond the feature names (read the jiff docs before designing), the effort estimate, and every recommendation
informs: [Q-118, Q-119]
---

# R-008: Moving from chrono and chrono-tz to jiff

## Why now
- RUSTSEC-2026-0543 (chrono) and RUSTSEC-2026-0555 (chrono-tz) were issued on 2026-11-17 as informational "unmaintained" advisories: no vulnerability, all versions affected, alternatives jiff or time. They are ignored in `deny.toml` (user approval, 2026-11-18).
- In chronotope/chrono#1768 (January 2026) the maintainer winds down both crates and recommends jiff. Asked whether chrono-tz would stop tracking time-zone database changes, he said yes. chrono-tz **compiles the rules into the binary**, so today a rule change reaches us only when chrono-tz publishes and we rebuild, and soon not at all.

## Where we use them
- chrono: 180 `.rs` files; every timestamp, date, "today" and API time field. Through features of sqlx (`chrono`), utoipa (`chrono`), ts-rs (`chrono-impl`), and via icalendar, rrule, croner and async-imap.
- chrono-tz: 11 crates (household time zone, calendar and weather conversion, task and routine days, meals).
- The web app formats times in the browser with `Intl`, which uses the device's own time-zone data (an OS or browser update), not ours.

## jiff 1.0
- Default features include `tz-system` and `tzdb-zoneinfo` (reads the system database, `/usr/share/zoneinfo`), `tzdb-bundle-platform` (a bundled copy on platforms without one), and `tzdb-bundle-always` exists to force the bundled copy.
- The runtime image is `debian:bookworm-slim`, which ships `tzdata` (2026c at the time of writing). Rules then update when the image is rebuilt with current Debian packages, or immediately if the host's `/usr/share/zoneinfo` is mounted read-only into the container.

## Ecosystem gaps
| Need | chrono today | jiff |
|---|---|---|
| Postgres columns (sqlx 0.8) | built-in `chrono` feature | `jiff-sqlx` 1.0.0 wrapper types (`postgres` feature); not native |
| OpenAPI (utoipa 5.5) | `chrono` feature | only `jiff_0_2`, not jiff 1.x: needs `value_type = String` overrides or a utoipa update |
| TypeScript types (ts-rs 11.1) | `chrono-impl` | none: needs `#[ts(type = "string")]` / `as` overrides |
| icalendar, rrule, croner | chrono types | stay chrono inside those crates; convert at the edge (`jiff-chrono-conversions` 0.2.0 exists) |

## Shapes of a migration
1. **Time zones first:** replace chrono-tz with jiff's time-zone database for every conversion, keep chrono types, convert at the edges. Fixes stale rules; a small part of the code.
2. **Full move:** jiff types everywhere, jiff-sqlx for columns, overrides for utoipa and ts-rs, chrono only inside third-party crates. Large and mechanical; touches nearly every crate.
3. **Stay:** keep both and rebuild when chrono-tz publishes, while it still does.
