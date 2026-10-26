---
id: Q-051
title: Email categories and what each one does
status: open
batch: 16
area: ingest
created: 2026-10-25
resolved:
decisions: []
---

# Q-051: Email categories and what each one does

## Question
`email.category` is a Choice over **the household's categories** (D-027). Each category needs a behaviour:
- `ignore`: kept as metadata only, no extraction, hidden by default;
- `keep`: listed, no extraction;
- `extract`: events, bills and tasks are extracted.

Each category also says whether a time-sensitive message in it sends a notification.

Proposed defaults:

| Category | Behaviour | Notify if time-sensitive |
|---|---|---|
| school | extract | yes |
| daycare | extract | yes |
| activities (clubs, sports, lessons) | extract | yes |
| appointments (medical, dental, services) | extract | yes |
| bills | extract | no (the pay task notifies) |
| household (utilities notices, deliveries, home services) | extract | no |
| personal | keep | no |
| newsletters | keep | no |
| promotions | ignore | no |
| social | ignore | no |
| junk | ignore | no |
| other | keep | no |

**Are these the right defaults?** Categories can be renamed, added or archived in Settings either way.

## Options
- A. **Use the proposed defaults, all editable in Settings** (recommended).
- B. A shorter starting list: school/daycare, activities, bills, other, junk.
- C. Start empty and add categories in the UI.
- D. Changes to the proposed list (describe which).
- E. Something else (describe).

## Answer
<!-- answer below this line -->
Option A.
