---
id: Q-119
title: Where the server gets its time-zone rules
status: open
batch: 26
area: infra
created: 2026-11-18
resolved:
decisions: []
---

# Q-119: Where the server gets its time-zone rules

## Context
Today the rules are compiled into `fa-server` by chrono-tz, so a change reaches a household only after chrono-tz publishes and we release. jiff reads `/usr/share/zoneinfo` by default and has a bundled copy as a fallback (R-008). The runtime image (`debian:bookworm-slim`) ships `tzdata`. Mounting the host's zoneinfo makes rule updates follow the host's OS updates; the server would log which source and version it loaded and show it on the Status page whichever option is chosen. Only matters if Q-118 is A or B.

## Question
With jiff the rules can come from the operating system's time-zone database instead of being compiled in (R-008). Where should the server read them from?

## Options
- A. Configurable, default: the container image's tzdata (Debian, refreshed on every release build), falling back to jiff's bundled copy when it is missing (recommended)
- B. Configurable, default: the host's /usr/share/zoneinfo mounted read-only into the container, so a host OS update fixes the rules without a new release; the image's copy is the fallback
- C. Always jiff's bundled copy: the same rules everywhere, updated only with a jiff release and our rebuild

## Answer
<!-- answer below this line -->

