---
id: D-055
title: Children get an optional gender and free-form relevance tags
status: accepted
date: 2026-10-25
source_questions: [Q-052]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Domain.md]
---

# D-055: Children get an optional gender and free-form relevance tags

## Context
PROJECT.md asks for school information to be filtered by age, grade and gender. Members had no gender field. Other filters (team, club, room, bus route) work the same way.

## Decision
Both:
- `member.gender`: optional, `female | male | other`, unset by default. Each value maps to a household-editable list of terms (default `female`: girls, girls', girl's; `male`: boys, boys', boy's). `other` and unset match no gendered terms.
- `member.relevance_tags[]`: free-form tags (`U10 soccer`, `Team Blue`, `Room 12`, `bus 7`).

A newsletter section or message that names a child's grade, class, tag or gender term links to that child. A section that names only the other gender's terms is collapsed for that child.

## Consequences
- Gender is optional and only used for this matching; it is never shown on the dashboard.
- Matching is deterministic in M3; per-child summaries come with the LLM in M4.

## Alternatives considered
- Gender field only.
- Tags only (the recommended option).
