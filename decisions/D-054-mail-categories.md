---
id: D-054
title: Default mail categories and behaviours, all editable
status: accepted
date: 2026-10-25
source_questions: [Q-051]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-054: Default mail categories and behaviours, all editable

## Context
`email.category` is a Choice over the household's categories (D-027). Each category needs a behaviour (`ignore | keep | extract`) and a notify flag.

## Decision
Seed the proposed set: school, daycare, activities, appointments (extract, notify when time-sensitive); bills and household (extract, no notify); personal, newsletters, other (keep); promotions, social, junk (ignore). Everything is editable in Settings → Mail → Categories: rename, add, archive, change behaviour.

## Consequences
`category` rows are seeded on household creation and exported to TOML. Built-in signals refer to category keys; archiving a category a built-in signal targets disables that signal.

## Alternatives considered
- A shorter starting list.
- Start empty.
