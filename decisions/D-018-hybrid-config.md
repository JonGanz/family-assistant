---
id: D-018
title: Hybrid configuration: file bootstrap/import-export, UI is source of truth thereafter
status: superseded
date: 2026-10-23
source_questions: [Q-017]
supersedes:
superseded_by: D-089
affects: [arch/Overall.md]
---

> **Superseded by [D-089](D-089-remove-household-toml.md).** The household TOML import and export were removed; the database and UI are the source of truth.

# D-018: Hybrid configuration: file bootstrap/import-export, UI is source of truth thereafter

## Context
The user picked option A.

## Decision
- `fa-server config import <file.toml>` seeds or merges household config. This covers members, orgs and contacts, sources (minus secrets), lists, rules, and location.
- After that, the DB is the source of truth, and the UI edits it.
- `config export` writes a redacted TOML file that the family can keep privately in version control. Secrets are never exported.
- The example config uses a made-up family (D-004).

## Consequences
- The config schema is versioned, and import validates against it.

## Alternatives considered
Files only, or UI only. Not chosen.
