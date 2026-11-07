---
id: D-102
title: Generic-text notifications through ntfy outside the LAN
status: accepted
date: 2026-11-07
source_questions: [Q-082]
supersedes: 
superseded_by:
affects: [arch/Platform.md]
---

# D-102: Generic-text notifications through ntfy outside the LAN

## Context
Notification titles can contain mail subjects and bill payees, which an ntfy server outside the LAN sees in clear.

## Decision
Notifications through ntfy have a 'generic text only' mode that sends a neutral title and body (for example 'New item in Family Assistant') and keeps the details in the app. Default: on when ntfy_url resolves outside the LAN and tailnet ranges, off otherwise; the household can override it in Settings → Notifications.

## Consequences
Needs the household setting and a LAN or tailnet check of ntfy_url. Item for the next milestone.

## Alternatives considered
B (plain toggle, default off), C (no such mode).
