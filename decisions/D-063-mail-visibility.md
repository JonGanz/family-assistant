---
id: D-063
title: Messages are visible to source owners, or all adults for household sources
status: accepted
date: 2026-10-25
source_questions: [Q-060]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Domain.md]
---

# D-063: Messages are visible to source owners, or all adults for household sources

## Context
Email is more personal than calendars.

## Decision
Each source has `visibility: owners | household`, default `owners`. Owners (or every adult for `household` sources) see its messages. Children never see the message list in M3, but see the events and tasks derived for them. The 'from email ↗' link works only for those who can see the message.

## Consequences
RLS-backed message queries filter by visible sources. Derived items stay visible under their own rules.

## Alternatives considered
- Every adult sees everything.
- Children also see org messages linked to them.
