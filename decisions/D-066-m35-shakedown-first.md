---
id: D-066
title: M3.5 is onboarding plus a real-hardware shakedown, before M4
status: accepted
date: 2026-10-26
source_questions: [Q-061]
supersedes: 
superseded_by:
affects: [arch/Overall.md, plan/M3.5.md, README.md]
---

# D-066: M3.5 is onboarding plus a real-hardware shakedown, before M4

## Context
The 2026-10-26 review found M0–M3 code-complete but never run on real data (H-02..H-11 open) and the Echo Show unproven. D-065 had already paused for onboarding as M3.5.

## Decision
Option A. M3.5 = the D-065 onboarding work **plus** the shakedown: deploy on the real server through the new install, do H-02..H-11 on it (one real mail source, one Google and one CalDAV calendar, the Echo Show, ntfy on phones), record approval-inbox volume and approve/reject ratios for a week, and fix the M3 known gaps. Then M4 (sized by D-067). M5 stays after M4.

## Consequences
- The first real install is also the first test of the new onboarding (H-13, H-14).
- The inbox measurements choose M4's first decision points (D-067).

## Alternatives considered
- B, M4 next: builds AI on unmeasured data.
- C, M5 in parallel: splits attention during the shakedown.
