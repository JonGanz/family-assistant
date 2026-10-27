---
title: Human steps (all milestones)
updated: 2026-10-26
---

# Human steps

This page answers "what is waiting on a person?" (P-02). There is one row per step. A step is done once it has been verified where the last column says. Under D-066, the M0 to M3 steps are redone on the **M3.5 install** (`docker compose` and the setup wizard) rather than through the old commands, so they are mostly unaffected by the old guide.

| Step | Milestone | What | Status | Verified in |
|---|---|---|---|---|
| H-01 | M0 | Create the public GitHub repo, push, confirm `ci.yml` runs | done (2026-10-24) | log 2026-10-24, *CI failure after H-01* |
| H-02 | M0 | Core node with Docker and the app running (now: H-13) | open | M3.5 shakedown |
| H-03 | M0 | Phone login; pair the Echo Show (now over plain HTTP, D-072) | open | M3.5 shakedown |
| H-04 | M0 | Restore rehearsal on the core node (now including the secrets volume) | open | M3.5 shakedown |
| H-05 | M1 | Google Cloud OAuth client; the redirect URI must be HTTPS, so on the tailnet address | open | M3.5 shakedown (after H-14) |
| H-06 | M1 | Location set (wizard), real calendars connected (Google, ICS, CalDAV) | open | M3.5 shakedown |
| H-07 | M1 | Echo Show demo: layout, readability at distance, overnight survival | open | M3.5 shakedown (also U-16, R-02) |
| H-08 | M2 | ntfy on phones, subscribed, test notification received | open | M3.5 shakedown |
| H-09 | M2 | Real lists, chores and a morning routine used for a week | open | M3.5 shakedown |
| H-10 | M3 | Gmail API scopes on the OAuth client, or an IMAP app password; reconnect | open | M3.5 shakedown |
| H-11 | M3 | Orgs, contacts and enrollments; one mailbox read-only; a week of inbox and labelling (R-04 may shrink it) | open | M3.5 shakedown |
| H-12 | M3.5 | Enable GHCR publishing, make the `fa-server` package public, push the first `v*` tag | open | M3.5 |
| H-13 | M3.5 | Fresh install on the real server from `deploy/README.md`, timed, wizard finished | open | M3.5 |
| H-14 | M3.5 | Tailscale auth key, `--profile tailscale`, phone on the tailnet, ntfy subscribed | open | M3.5 |
