---
id: Q-068
title: Who may run the first-run wizard?
status: resolved
batch: 18
area: security
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-073]
---

# Q-068: Who may run the first-run wizard?

## Question
The wizard appears when no household exists, and creates the first adult (who can then do everything). Whoever reaches it first owns the install. Home Assistant and Jellyfin leave onboarding open to the first visitor; others print a one-time code in the container log, which brings back a command-line step the user wants gone.

**How is first-run setup protected?**

## Options
- A. **Configurable, default: open only to clients on private, loopback or tailnet (100.64.0.0/10) addresses, and only until the first adult exists**; `FA_SETUP_CODE` (or a generated code printed in the log when `FA_SETUP_REQUIRE_CODE=true`) can be required (recommended).
- B. Always require a one-time setup code, printed in the log (`docker compose logs app`) and shown on the server console.
- C. Open to the first visitor, no restriction (Home Assistant style).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C, this is a self-hosted application.
