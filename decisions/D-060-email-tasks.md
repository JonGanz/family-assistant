---
id: D-060
title: Email tasks are suggested by default, configurable per category
status: accepted
date: 2026-10-25
source_questions: [Q-057]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-060: Email tasks are suggested by default, configurable per category

## Context
PROJECT.md lists responding to emails as a task type. M3 has deterministic signals only (action phrases, questions from known contacts).

## Decision
Each category has `email_tasks: suggest | create | off`, default `suggest`. Action phrases and contact questions produce **suggested** tasks (status `suggested`) in the approval inbox, with assignees pre-filled from the member links (adults for reply tasks). 'Make task' on any message is always available.

## Consequences
Suggested tasks live on a list of kind `email` and are invisible elsewhere until accepted.

## Alternatives considered
- By hand only.
- Created automatically.
