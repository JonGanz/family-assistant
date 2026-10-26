---
id: D-010
title: Remote access via Tailscale; Android notifications via self-hosted ntfy
status: accepted
date: 2026-10-23
source_questions: [Q-009]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

> Extended by D-064 (2026-10-26): an optional dashboard-only listener on the home network for screens that cannot run Tailscale. Everything else stays tailnet-only.

# D-010: Remote access via Tailscale; Android notifications via self-hosted ntfy

## Context
The user wants Tailscale for remote access. Android notifications for new or priority tasks and for urgent emails are a nice-to-have.

## Decision
- The core node joins the tailnet. `tailscale serve` provides HTTPS to the web app, and nothing is exposed to the public internet.
- Notifications go through a **self-hosted ntfy** container on the core node, reached over the tailnet, using the ntfy Android app. Each member has their own topic. There are triggers for: a new priority task, a time-sensitive email, an approval needed, and a source that needs re-auth.
- Web Push from the PWA can be added later behind the same `Notifier` trait.

## Consequences
- Phones must be on the tailnet to get notifications.
- Notification rules are part of each member's settings.

## Alternatives considered
Cloudflare Tunnel (public exposure) and FCM (cloud dependency). Not chosen.
