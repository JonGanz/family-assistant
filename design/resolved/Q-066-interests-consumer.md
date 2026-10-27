---
id: Q-066
title: What should use member interests, hobbies and responsibilities?
status: resolved
batch: 17
area: domain
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-071]
---

# Q-066: What should use member interests, hobbies and responsibilities?

## Question
PROJECT.md says members' interests, hobbies and responsibilities "help guide the intelligent decision making". They are imported and stored (fa-config household import) but no code reads them (review risk R-07). Candidate consumers: mail relevance (treat them like the D-055 relevance tags), the Now-list score (D-042), M6 smart assignment (D-050), and M4 System One features.

**Which consumer(s) first?**

## Options
- A. **Mail relevance now (M3.5): interests and hobbies act as extra relevance tags per member; responsibilities feed M6 smart assignment later** (recommended).
- B. Wait for M4 and let System One features consume them.
- C. Wait for M6 smart assignment; interests stay informational until then.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
They would be used strictly by the LLM features to drive automations.
