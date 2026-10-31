---
id: D-091
title: The dashboard shows no heartbeat time; the connection status absorbs it
status: accepted
date: 2026-10-30
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-091: The heartbeat is part of the connection status

## Context
Direct user instruction (2026-10-30, no question), refining D-085: remove the "Last heartbeat" line from the dashboard and fold it into the connection status.

## Decision
- The bar shows only the connection status. The heartbeat time is no longer displayed.
- The status reads **Connected** only while the stream is open and alive: the server sends a heartbeat every 15 s, and an open stream with no heartbeat for 45 s (three intervals) reads **No data** (warning colour, `data-status="stalled"`). The other states are unchanged: Connecting, Reconnecting, Offline.
- The kiosk watchdog (reload after 120 s without a heartbeat) is unchanged.
