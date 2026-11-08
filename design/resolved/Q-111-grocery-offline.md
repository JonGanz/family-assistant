---
id: Q-111
title: Shopping with poor signal
status: resolved
batch: 24
area: frontend
created: 2026-11-07
resolved: 2026-11-08
decisions: [D-118]
---

# Q-111: Shopping with poor signal

## Context
Phones in a store often lose signal. Over plain HTTP the service worker is off (D-072, Frontend.md §1.1), so an offline reload cannot load the app; an open page can still keep working.

## Question
How does the grocery list behave offline?

## Options
- A. An open list keeps working: adds and check-offs queue locally and sync when back, merged by item version; an "offline, N changes waiting" note shows (suggested)
- B. Online only, with a clear offline note
- C. A, plus a full offline app when served over HTTPS (service worker caches the list page)

## Answer
<!-- answer below this line -->
Option C.
