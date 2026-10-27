---
id: Q-064
title: Which peers may set the client IP (X-Forwarded-For)?
status: open
batch: 17
area: security
created: 2026-10-26
resolved:
decisions: []
---

# Q-064: Which peers may set the client IP (X-Forwarded-For)?

## Question
Review finding C-04. `fa-api/src/extract.rs` trusts `X-Forwarded-For` from loopback and all of `172.16.0.0/12` by default, so that the Docker bridge works out of the box. Any other container on the bridge, or a home LAN that happens to use 172.16–31.x, can therefore spoof client IPs and get around the per-IP login limiter. The deployment is Tailscale-only except the D-064 dashboard listener.

**What should the default be?**

## Options
- A. **Loopback only, plus an explicit `FA_TRUSTED_PROXIES` list; the compose file sets it to the Docker gateway** (recommended).
- B. Keep `172.16.0.0/12` as the default for zero-config Docker; document the risk.
- C. Detect the Docker gateway at startup and trust only that address, with the env list as an override.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option A.
