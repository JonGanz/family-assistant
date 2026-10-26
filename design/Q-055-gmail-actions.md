---
id: Q-055
title: What may the system do in Gmail when a source isn't read-only?
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-055: What may the system do in Gmail when a source isn't read-only?

## Question
D-007: sources are read-only by default. Turning that off grants `gmail.modify` and allows these actions: archive, mark read, and apply `FA/*` labels. All are logged and can be undone. The question is which of them happen **automatically** and which only on a click.

**What should run automatically on a writable source?**

## Options
- A. Nothing automatic. Actions only from the message view.
- B. Labels only: apply `FA/<category>` (and `FA/<child>`) labels so Gmail shows the classification.
- C. **Configurable per category, default B** (recommended): labels by default, plus optional per-category 'archive after processing' or 'mark read' (for example, archive junk and promotions). Every automatic action is undoable from the message view for 30 days.
- D. Labels plus auto-archive of `ignore` categories, out of the box.
- E. Something else (describe).

## Answer
<!-- answer below this line -->
Option C.
