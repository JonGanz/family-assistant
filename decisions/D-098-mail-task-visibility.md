---
id: D-098
title: Mail-derived tasks follow the mailbox per source, default until accepted
status: accepted
date: 2026-11-07
source_questions: [Q-078]
supersedes: 
superseded_by:
affects: [arch/Tasks.md, arch/Ingest.md]
---

# D-098: Mail-derived tasks follow the mailbox per source, default until accepted

## Context
Code review 2 (C2-21): tasks created from mail skip the mailbox visibility, so any adult can list, edit or delete a suggested or open task derived from another adult's owners-only mailbox, and its title is the mail subject.

## Decision
Configurable per mail source, default: a mail-derived task follows its source mailbox's visibility while it is suggested, and becomes an ordinary household task once accepted. The setting is a per-source choice with the values `follow_until_accepted` (default), `follow_until_done` and `household`; it is edited in Settings → Mail on the source. Task list, edit and delete routes apply the same visibility check as skip and accept.

## Consequences
Needs a column on the mail source (written by the mail source routes and Settings → Mail) and a visibility filter in the task queries. Planned as a LARGER item of code review 2 for the next milestone.

## Alternatives considered
A (fixed: household once accepted), B (always follow the mailbox), C (always household). The user picked D, configurability with A as the default.
