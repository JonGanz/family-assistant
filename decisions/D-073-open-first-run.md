---
id: D-073
title: The first-run wizard is open to the first visitor
status: accepted
date: 2026-10-26
source_questions: [Q-068]
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md]
---

# D-073: The first-run wizard is open to the first visitor

## Context
The wizard creates the first adult, who controls the install.

## Decision
Option C. No address guard and no setup code: whoever opens the app first while no household exists runs the wizard (Home Assistant style). Setup closes for good once the first adult account exists.

## Consequences
- The hoster should finish setup right after `docker compose up`; the install guide says so.
- No `FA_SETUP_CODE` or `FA_SETUP_REQUIRE_CODE`.

## Alternatives considered
- A, private-address guard with an optional code.
- B, an always-required code from the log.
