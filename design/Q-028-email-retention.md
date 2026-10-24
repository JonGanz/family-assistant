---
id: Q-028
title: Email content retention & privacy
status: open
batch: 5
area: security
created: 2026-10-23
resolved:
decisions: []
---

# Q-028: Email content retention & privacy

## Question
The system copies email from Gmail. How much should it keep, and for how long?
- **Body text:** needed for re-processing, kNN, and the eval sample. It is the most sensitive data we hold.
- **Attachments:** only the extracted text, or the files themselves?
- **Retention:** for example, keep bodies 90 days for non-actionable mail (junk, promos) and indefinitely for mail linked to events, bills, or tasks. Gmail stays the system of record either way.

## Options
- A: keep body text; drop bodies of junk/promo after 30 days, others after 1 year unless linked to an event/bill/task/eval sample; attachments: extracted text only (recommended)
- B: never store bodies, only headers/snippet + extracted results (re-fetch from Gmail when needed)
- C: keep everything indefinitely

## Answer
<!-- answer below this line -->

