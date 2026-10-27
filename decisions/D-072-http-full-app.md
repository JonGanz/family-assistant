---
id: D-072
title: Plain HTTP serves the entire app; HTTPS is the hoster's choice
status: accepted
date: 2026-10-26
source_questions: [Q-067]
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md]
---

# D-072: Plain HTTP serves the entire app; HTTPS is the hoster's choice

## Context
D-065 made HTTPS optional. The question was what to serve on the home network without it.

## Decision
The **entire app** is available over plain HTTP: login, settings, PIN elevation, everything. Skipping HTTPS is the hoster's choice; the app does not nag about it. Settings, System shows the connection type as a fact, with no banners on the login page. There is no new `lan_mode` setting. The D-064 dashboard-only listener stays as an existing opt-in for anyone who wants a restricted LAN port.

## Consequences
- Passwords, PINs and cookies cross the LAN unencrypted when HTTPS is skipped; that is accepted by design.
- The web app must work in an insecure context (no service worker, no `crypto.subtle`, no `crypto.randomUUID`, no async clipboard).
- Google sign-in still needs HTTPS (Google's rule).

## Alternatives considered
- B, dashboard-only on HTTP.
- C, passwords only from the server itself.
