---
id: Q-067
title: Without Tailscale, what is reachable on the home network over plain HTTP?
status: resolved
batch: 18
area: security
created: 2026-10-26
resolved: 2026-10-26
decisions: [D-072]
---

# Q-067: Without Tailscale, what is reachable on the home network over plain HTTP?

## Question
D-065 makes HTTPS optional. Today only the dashboard-only listener (D-064) is on the LAN, and logins happen over the tailnet. Over plain HTTP on the LAN, passwords, PINs and session cookies are unencrypted, and anyone on the Wi-Fi (guests, a compromised smart device) could sniff them. The app still must work for a household that never installs Tailscale.

**What should be served on the home network when HTTPS is not available?**

## Options
- A. **Configurable, default: the full app on the LAN over HTTP**, with a persistent "not encrypted" notice in Settings and on the login page, and PIN elevation allowed (recommended). The D-064 dashboard-only listener stays as an opt-in hardening mode (`lan_mode = full | dashboard_only | off`).
- B. Configurable, default dashboard-only (D-064 as today): logins and settings need Tailscale or HTTPS. Safer, but a household without Tailscale can't log in, which defeats the point.
- C. Full app on the LAN, but passwords and PINs refused over HTTP unless the client is on the server itself (setup from the server's own browser only).
- D. Something else (describe).

## Answer
<!-- answer below this line -->
It's self-hosted. If a user wishes to skip HTTPS, that is their choice. The entire app should remain avialable.
