---
id: Q-070
title: Which server settings move from environment variables into the UI?
status: resolved
batch: 18
area: tech
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-075]
---

# Q-070: Which server settings move from environment variables into the UI?

## Question
D-065 removes the TOML file and admin commands from setup, but about 20 `FA_*` variables remain (README, Configuration reference). Some are infrastructure (database URL, master key path, bind address, log level); others are things a household changes later: public address, ntfy server and token, Google client ID and secret, plain-HTTP feeds, the D-064 LAN dashboard, trusted proxies. D-026 and D-029 already say behaviour that varies should be editable in the UI.

**Where should those settings live?**

## Options
- A. **Everything except infrastructure moves to a Settings, System page** (stored in the DB, secrets envelope-encrypted). An environment variable still overrides it and the field then shows "set by the server" and is locked (recommended).
- B. Only what the wizard needs (public address, notifications on/off); integrations (Google, ntfy token, LAN dashboard) stay in `.env`.
- C. Everything stays in `.env`; the wizard only does household data.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Definitely option A.
