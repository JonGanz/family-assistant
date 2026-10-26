---
id: Q-063
title: What may an unelevated dashboard device do: read the approval inbox, dismiss advisories?
status: open
batch: 17
area: security
created: 2026-10-26
resolved:
decisions: []
---

# Q-063: What may an unelevated dashboard device do: read the approval inbox, dismiss advisories?

## Question
Review findings C-08 and C-09. In `fa-domain/src/authz.rs`, `ReadInbox` is allowed for any device, elevated or not, so a kiosk token (including one on the home-network listener, D-064) shows pending mail-derived approvals. `DismissAdvisory` is allowed for everyone, children and unelevated devices included, so a weather advisory can be dismissed before the adults see it.

**What should the defaults be?**

## Options
- A. **Configurable per device and per household, defaults: inbox needs elevation, advisories dismissable by adults and elevated devices only** (recommended).
- B. Keep as is: the kiosk is in the home, so the inbox summary and dismissals are fine for everyone.
- C. Inbox needs elevation; advisories stay dismissable by anyone (they are low-stakes).
- D. Something else (describe).

## Answer
<!-- answer below this line -->

