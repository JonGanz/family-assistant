---
id: D-075
title: Every non-infrastructure server setting is editable in Settings, System
status: accepted
date: 2026-10-26
source_questions: [Q-070]
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md]
---

# D-075: Every non-infrastructure server setting is editable in Settings, System

## Context
About 20 `FA_*` variables remained after D-065.

## Decision
Option A. Everything except infrastructure (database URL, secrets location, bind address, log level) moves to **Settings, System**: public address(es), ntfy URL and token, Google client ID and secret, plain-HTTP feeds, private hosts for feeds (C-05), the D-064 listener, trusted proxies (D-069). Stored in the DB (secrets envelope-encrypted). An environment variable still overrides and locks its field ('set by the server').

## Consequences
- Settings reload without restart where possible (pg_notify); listener and bind changes say 'restart needed'.
- Refines Platform.md §6 configuration layers.

## Alternatives considered
- B, only wizard settings in the UI.
- C, everything in `.env`.
