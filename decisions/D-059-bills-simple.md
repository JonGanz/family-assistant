---
id: D-059
title: Bills: pay tasks go to the adult pool, no autopay handling, paid when the task is done
status: accepted
date: 2026-10-25
source_questions: [Q-056]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-059: Bills: pay tasks go to the adult pool, no autopay handling, paid when the task is done

## Context
D-009 itemises bills with a 'Pay <payee>' task. Open were the assignee, autopay and payment detection.

## Decision
- The pay task uses the **pool** strategy over all adults (Tasks.md).
- No autopay flag: every accepted bill gets a pay task.
- A bill is `paid` only when its pay task is completed (undo reopens it). Payment-confirmation emails are not matched; a 'payment received' phrase rule sets `is_bill = false` so they don't create bills.

## Consequences
The `bill` table has no `autopay` column. A household that wants per-biller assignees or autopay can raise it later as a new question.

## Alternatives considered
- Per-biller assignee, autopay reminders and payment-email matching (recommended option B).
- Autopay bills with no task.
