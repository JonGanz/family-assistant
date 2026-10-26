---
id: D-056
title: New org contacts are suggested by default, configurable per org
status: accepted
date: 2026-10-25
source_questions: [Q-053]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-056: New org contacts are suggested by default, configurable per org

## Context
Teachers and coaches change every year, so entering contacts by hand goes stale.

## Decision
Each org has `contact_mode: suggest | auto | manual`, default `suggest`. An unknown sender from a known org's domain who mentions a child's name, alias, grade or class becomes a **contact proposal** in the approval inbox ('<name> at <org> → <child>?'). One tap creates the contact and its enrollment link. `auto` creates it directly; `manual` never proposes.

## Consequences
The inbox gains a `contact` kind. A rejected suggestion is remembered per address so it isn't proposed again.

## Alternatives considered
- Manual only.
- Automatic only.
