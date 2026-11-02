---
id: Q-082
title: A generic-text mode for notifications sent through ntfy
status: open
batch: 21
area: notify
created: 2026-11-02
resolved:
decisions: []
---

# Q-082: A generic-text mode for notifications sent through ntfy

## Question
Notification titles can contain mail subjects and bill payees, which an ntfy server outside the LAN sees in clear. Should there be a 'generic text only' mode that sends a neutral title and body and keeps the details in the app?

## Options
- A. Yes; default on when ntfy_url is not on the LAN or tailnet, off otherwise (suggested default)
- B. Yes, a plain toggle, default off
- C. No

## Answer
<!-- answer below this line -->

