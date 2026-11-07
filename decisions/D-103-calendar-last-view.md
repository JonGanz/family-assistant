---
id: D-103
title: Calendar view switch under the top bar; reopen the last used view
status: accepted
date: 2026-11-07
source_questions: [Q-087]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-103: Calendar view switch under the top bar; reopen the last used view

## Context
Q-087: the Calendar's Agenda, Week and Month switch does not fit in the top bar at 360 px. M4 (T-10) put it in a row under the top bar, with Agenda the default on phones and Week on tablets and desktop.

## Decision
The view switch stays in a row under the top bar. The first visit opens Agenda on compact widths and Week on medium and expanded widths. After that, the Calendar reopens in the view the person last used on that device (stored per device, key `fa.calendar.view`). There is no setting for it.

## Consequences
A small web change (T-20). The per-width default applies only when nothing is stored.

## Alternatives considered
B (overflow menu), C (a per-device setting: the user found remembering the last view enough, without an extra option).
