---
id: D-070
title: Members are managed on an adult-only Settings screen; children see members on the dashboard
status: accepted
date: 2026-10-26
source_questions: [Q-065]
supersedes: 
superseded_by:
affects: [arch/Frontend.md, arch/Domain.md]
---

# D-070: Members are managed on an adult-only Settings screen; children see members on the dashboard

## Context
Review finding U-08: `/members` was a top-level management page for every signed-in user.

## Decision
- Member management (add, edit, avatar, role, accounts, PINs, interests and responsibilities) is an **adult-only Settings screen**, `/settings/members`. **No redirect** from `/members`; the old route is removed.
- Only adults edit member details, because other features rely on them.
- Children see the household's members **on the dashboard**, where the family views shared information. The app has no members page for children.

## Consequences
- Fits the Settings grouping of U-08 (Household group).
- A child's own profile page, if wanted later, is a new question.

## Alternatives considered
- A, a read-only Household card for children on Today.
- C, keep `/members` top-level.
