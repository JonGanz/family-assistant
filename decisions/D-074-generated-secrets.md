---
id: D-074
title: Secrets are generated on first start; the wizard requires a master-key backup
status: accepted
date: 2026-10-26
source_questions: [Q-069]
supersedes: 
superseded_by:
affects: [arch/Platform.md, arch/Frontend.md]
---

# D-074: Secrets are generated on first start; the wizard requires a master-key backup

## Context
Hosters made the master key by hand and invented the database password in `.env`.

## Decision
Option A. A one-shot `init` service creates the master key and the database password in a `secrets` volume if absent (Postgres reads `POSTGRES_PASSWORD_FILE`). The wizard has a required step to copy or download the key, and Settings, System warns until an adult confirms a backup. Hoster-provided files still work.

## Consequences
- Losing the volume without the backup loses every stored sign-in; `restore.md` covers the volume.

## Alternatives considered
- B, no wizard step.
- C, hoster-made files with a better guide.
