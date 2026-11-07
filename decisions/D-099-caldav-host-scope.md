---
id: D-099
title: CalDAV follows hrefs within the source's registrable domain, per-source override
status: accepted
date: 2026-11-07
source_questions: [Q-079]
supersedes: 
superseded_by:
affects: [arch/Feeds.md]
---

# D-099: CalDAV follows hrefs within the source's registrable domain, per-source override

## Context
Code review 2 (C2-15): the CalDAV client follows server-supplied hrefs to any host and sends Basic credentials there. iCloud legitimately redirects to shards such as pNN-caldav.icloud.com.

## Decision
A CalDAV client follows hrefs only within the configured source's registrable domain (public-suffix aware). A per-source toggle 'Trust other hosts' (default off, Settings → Calendars on the source) lifts the restriction. Credentials are never sent to a host outside the allowed set; such an href is reported as a sync error.

## Consequences
Needs a public-suffix lookup and a source column for the toggle. LARGER item for the next milestone.

## Alternatives considered
B (configured origin only: breaks iCloud shards), C (any host: leaks credentials).
