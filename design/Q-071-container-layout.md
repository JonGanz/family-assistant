---
id: Q-071
title: How many app containers does the default compose file run?
status: open
batch: 18
area: infra
created: 2026-10-26
resolved:
decisions: []
---

# Q-071: How many app containers does the default compose file run?

## Question
Today the compose file runs `api` and `worker` from the same image, plus postgres, ntfy and backup. `fa-server serve all` runs both roles in one process and is already used in dev and e2e.

**What should the default be?**

## Options
- A. **Configurable, default one `app` container (`serve all`)**; splitting into api and worker is a documented compose override for larger installs (recommended).
- B. Keep api and worker as two containers by default.
- C. Something else (describe).

## Answer
<!-- answer below this line -->

