---
id: D-058
title: Automatic mail actions are configurable per category, default labels only
status: accepted
date: 2026-10-25
source_questions: [Q-055]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-058: Automatic mail actions are configurable per category, default labels only

## Context
D-007 allows archive, mark read and labels on writable sources. The question was which run without a click.

## Decision
On a writable source, each category has `auto_actions`: by default `label` only (`FA/<category>` and `FA/<child>`). Per category an adult can add `archive` or `mark_read` (for example, archive junk). Every automatic action is logged in `gmail_action` and can be undone from the message view for 30 days.

## Consequences
Read-only sources (the default, D-007) never get actions. On IMAP, labels are Gmail labels or IMAP keywords (D-052).

## Alternatives considered
- Nothing automatic.
- Auto-archive of `ignore` categories out of the box.
