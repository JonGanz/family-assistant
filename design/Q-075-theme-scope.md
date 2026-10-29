---
id: Q-075
title: Who chooses the theme: household, member, or device?
status: open
batch: 19
area: frontend
created: 2026-10-29
resolved:
decisions: []
---

# Q-075: Who chooses the theme: household, member, or device?

## Context
The user has said the kiosk (`/dashboard`) must use the same theme as the rest of the app, not a separate dark default. Burn-in shifting on the kiosk stays as it is (`arch/Frontend.md` §4); it does not depend on the theme. A kiosk is a device, not a signed-in person, so "per member" cannot apply to it.

## Question
**At which level is the theme chosen?**

## Options
- A. **A household default, set by an adult in Settings. The kiosk and any signed-out device use it. A signed-in member may override it for their own phone or desktop** (recommended).
- B. Household only: one theme for everyone and every screen.
- C. Household default, with an override per member and per device (including the kiosk).
- D. **Configurable, default A:** a household setting turns member overrides off (B) or adds per-device overrides (C).
- E. Something else (describe).

## Answer
<!-- answer below this line -->
