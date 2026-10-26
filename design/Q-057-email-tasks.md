---
id: Q-057
title: Tasks from email: 'reply', 'sign', 'RSVP'
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-057: Tasks from email: 'reply', 'sign', 'RSVP'

## Question
PROJECT.md lists "responding to emails" as a task type, and tasks can be auto-populated from emails. Without an LLM (M3), the deterministic signals are:
- action phrases ("please sign", "permission slip", "RSVP by", "reply by", "return the form by"), with a deadline when one is found;
- a direct question from a known contact (a teacher) addressed to you.

These go to a list of kind `email`.

**How should M3 create email tasks?**

## Options
- A. Only by hand: a 'Make task' button on any message.
- B. Automatically from action phrases, plus by hand.
- C. **Configurable, default 'suggest'** (recommended): action phrases and contact questions create *suggested* tasks in the approval inbox (one tap to accept, with the assignee pre-filled from the member links). The 'Make task' button is always there. A category can be switched to 'create' (B) or 'off' (A).
- D. Something else (describe).

## Answer
<!-- answer below this line -->

