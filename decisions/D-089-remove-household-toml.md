---
id: D-089
title: Remove the household TOML import and export
status: accepted
date: 2026-10-30
source_questions: [[, ]]
supersedes: D-018
superseded_by:
affects: [arch/Overall.md, arch/Platform.md, arch/Frontend.md, arch/Feeds.md, arch/Ingest.md, arch/Tasks.md]
---

# D-089: Remove the household TOML import and export

## Context
Direct user instruction (2026-10-30, not a question): the household TOML was legacy. Since the first-run wizard (D-065) and the UI-owned settings (D-075) it was only an optional upload and download, and every section of it already maps to a database table. The user asked to remove TOML usage from the backend and update the README.

## Decision
- Remove `fa_config::{import, export, HouseholdConfig}`, the `fa-config` TOML schema, `POST /api/v1/config/import`, `GET /api/v1/config/export`, `fa-server config import|export`, the Settings, System "Household file" card and the wizard's import option, and `config/examples/household.example.toml`.
- The database and the UI are the only source of household data. There is no file backup of the household in the app; use a database backup (runbooks/restore.md).
- The optional process-level `fa.toml` (figment, env > `fa.toml` > default for process and runtime settings, D-075) is **not** household data and stays.
- `fa-server user add|pin set|notify test --member` now take the member's display name (case-insensitive), because there are no TOML member keys any more. The `member.config_key` column stays (a deployed migration is never edited) but nothing writes it.
- The development and e2e example household is created by `e2e/seed-household.mjs` through the setup API, replacing `config import`.

## Consequences
- Supersedes D-018 (hybrid configuration). The import and export parts of D-029 go with it.
- Households can no longer be moved or versioned as a file. A different export (for example JSON) would need a new decision.
- The disabled example "School calendar" source and the org calendar links of the old example file are not seeded (the API needs an ICS URL).

## Alternatives considered
Keeping TOML as a recovery and scripting tool (CLI only), or replacing it with JSON. Not chosen: the user called it legacy.
