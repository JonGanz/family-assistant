---
id: D-011
title: Echo Show 15 dashboard via Silk browser with keep-alive; touch-enabled, no Alexa integration
status: accepted
date: 2026-10-23
source_questions: [Q-010]
supersedes:
superseded_by:
affects: [arch/Overall.md, arch/Frontend.md (future)]
---

> **Amended 2026-10-24 (user feedback):** the pairing link uses a query parameter (`/dashboard?device=…`) instead of the URL fragment. To compensate: the token is stripped from the URL on load, logs record only the path, and `Referrer-Policy: no-referrer` is sent. The dashboard also shows the mouse cursor (needed for non-touch screens).

# D-011: Echo Show 15 dashboard via Silk browser with keep-alive; touch-enabled, no Alexa integration

## Context
The user picked the Silk keep-alive approach and doesn't want Alexa or voice control.

## Decision
- The dashboard is the `/dashboard` route of the Vue PWA, opened in the Echo Show's Silk browser. It is touch-enabled (see D-006 and D-019).
- The keep-alive and resilience measures are:
  - Request the Screen Wake Lock API, and fall back to a muted, looping, 1-px inline video.
  - Keep the page active so Silk doesn't time out to the home screen. A small periodic DOM or animation tick does this.
  - Reconnect the SSE stream with backoff and run a full-reload watchdog (for example, reload if there has been no heartbeat for 2 minutes, plus a nightly reload).
  - The device token lives in localStorage, with a fallback in the URL fragment (`/dashboard#device=...`), so a cleared storage re-pairs itself.
  - A service-worker cache shows the last known state if the server can't be reached.
- The layout is designed for 1920×1080 landscape or portrait (configurable) at viewing distance.

## Consequences
- Silk may still exit after Fire OS updates. If the keep-alive turns out to be too fragile, that gets its own question; we won't switch devices on our own.
- No Alexa skill.

## Alternatives considered
Sideloading Fully Kiosk, or a different device. Declined for now.
