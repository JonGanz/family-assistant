---
id: D-031
title: Children sign in with an adult-set username + simple password; long-lived device session
status: accepted
date: 2026-10-23
source_questions: [Q-029]
supersedes:
superseded_by:
affects: [arch/Platform.md, arch/Domain.md]
---

# D-031: Children sign in with an adult-set username + simple password; long-lived device session

## Context
The user picked option A.

## Decision
- An adult creates the child's `UserAccount` and sets or resets its password.
- Child passwords may be short (at least 4 characters). Adult passwords need at least 12. Both have the same rate limits and lockout.
- Child sessions roll for 180 days (adults 30). An adult can revoke any child session from settings.

## Consequences
- The weaker password is acceptable because of the network boundary (tailnet only, D-010) and because child accounts have narrow permissions (D-006).

## Alternatives considered
Device pairing or dashboard-only access. Not chosen.
