---
id: D-064
title: Optional dashboard-only listener on the home network; everything else stays tailnet-only
status: accepted
date: 2026-10-26
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md]
---

> **Widened by [D-065](D-065-simple-onboarding.md) (2026-10-26):** the full app may be reachable on the home network over HTTP; this listener becomes an optional hardening mode (see Q-067).

# D-064: Optional dashboard-only listener on the home network; everything else stays tailnet-only

## Context
The wall screen the household has in mind (an Echo Show, D-011) cannot run Tailscale, so it cannot open the tailnet address (D-010). The user asked (2026-10-26, in direct instruction rather than a question) for the dashboard to be reachable on the local network, with everything else kept as secure as possible.

## Decision
- `serve api` can open a **second listener for the dashboard only**. It is off unless `FA_LAN_DASHBOARD_URL` is set. The compose file publishes it on a host address you choose (`FA_LAN_DASHBOARD_HOST_IP`, default `127.0.0.1`, which keeps it unreachable).
- On that listener:
  - Only **device tokens** are accepted. Session cookies are dropped, and a user's bearer token is refused.
  - Only the API calls the dashboard makes are routed (an allowlist), plus the static web app and the health checks. Everything else is 404.
  - Only clients in `FA_LAN_ALLOWED_CIDRS` are served (default: private and loopback ranges). `X-Forwarded-For` is ignored.
  - **PIN elevation is off** unless `FA_LAN_ALLOW_PIN=true`, because the listener is plain HTTP and a PIN would cross the network unencrypted.
- Pairing shows a second link (and QR code) for the home network when the listener is on.

## Consequences
- An Echo Show or any browser on the LAN can show the dashboard. Login, settings, mail and every adult or child page stay reachable only over Tailscale.
- The device token travels unencrypted on the LAN. Anyone who can sniff the Wi-Fi could reuse it to read what the wall screen shows, and to tick tasks if the household allows that. Revoking the device (Settings, Devices) ends this.
- With elevation off, approvals and adult-only actions are not available on a LAN screen. Ticking tasks still works under the household's dashboard-completion setting.
- Configurable, default off. Configurability is the user's stated default.

## Alternatives considered
- **HTTPS on the LAN**, with the tailnet certificate behind a local DNS override, or a private CA. Browsers like Silk do not trust a private CA, and the DNS override is beyond a newcomer. Left open as a later option.
- **Exposing the whole app on the LAN.** Rejected: it widens the attack surface well beyond what the screen needs.
- **A Tailscale subnet router.** It does not help, because it routes from the tailnet to the LAN, not from the LAN to the tailnet.
