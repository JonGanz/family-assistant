---
id: D-118
title: "Grocery list works offline: local queue, and a full offline app over HTTPS"
status: accepted
date: 2026-11-08
source_questions: [Q-111]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md]
---

# D-118: Grocery list works offline: local queue, and a full offline app over HTTPS

## Context
Phones lose signal in stores. Over plain HTTP the service worker is off (D-072). Answer: option C.

## Decision
An open Shopping List keeps working without a connection: adds and check-offs queue in local storage and replay when back, merged by item `version` (a check-off wins over a concurrent edit; a deleted item drops its queued changes); an "offline, N changes waiting" note shows. When served over HTTPS (secure context), the service worker also caches the app shell and the last list, so the Food view opens offline.

## Consequences
Client ids for queued adds come from `lib/ids` (D-072). The API accepts a client-generated item id so a replayed add is idempotent.

## Alternatives considered
Online only (Q-111 B); the queue without the service worker (Q-111 A).
