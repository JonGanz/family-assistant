---
id: D-104
title: M4 applied choices confirmed (Q-088, Q-089, Q-093 to Q-097)
status: accepted
date: 2026-11-07
source_questions: [Q-088, Q-089, Q-093, Q-094, Q-095, Q-096, Q-097]
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/AI.md]
---

# D-104: M4 applied choices confirmed (Q-088, Q-089, Q-093 to Q-097)

## Context
Batch 23 (Q-088, Q-089, Q-093 to Q-097) recorded the choices Claude applied while M4 ran, for the user to confirm.

## Decision
Confirmed as applied:
- Q-088: Today shows a Now card with the viewer's own Now tasks, ticked off with a 10 s Undo, hidden when empty.
- Q-089: Inbox Approve applies instantly; Reject asks for confirmation (no undo endpoint yet).
- Q-093: members are edited in a sheet on every width; Settings → Account changes the password through the Save bar. Accepted 'for now'; it can be revisited.
- Q-094: Settings → System switches are part of the page draft and save with the Save bar, since each save re-reads the server config.
- Q-095: `fa-server eval --with-llm` uses the stored llm stage threshold even when the stage is off, else 0.7; a failed LLM call counts as undecided.
- Q-096: Settings → AI sits under 'Sources and rules' after Mail; a newly saved endpoint starts enabled.
- Q-097: AI-made inbox items carry 'AI · NN%' ('AI' for tasks), a tooltip on wide screens and a sheet on phones, and the Inbox has an 'AI only' filter chip that is not remembered.

## Consequences
No code change; the M4 implementation stands. Q-089 option D (undo endpoints) remains a possible follow-up.

## Alternatives considered
The alternatives listed in each question; none was chosen.
