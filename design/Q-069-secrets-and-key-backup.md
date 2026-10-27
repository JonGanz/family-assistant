---
id: Q-069
title: How are the master key and database password created, and how is the key backed up?
status: open
batch: 18
area: security
created: 2026-10-26
resolved:
decisions: []
---

# Q-069: How are the master key and database password created, and how is the key backed up?

## Question
Today the hoster makes `secrets/fa_master_key` with `head -c 32 /dev/urandom`, sets file modes, and invents `POSTGRES_PASSWORD` in `.env`. The master key encrypts OAuth tokens and app passwords; it is never in the DB or backups (Platform.md §5, §9), so losing it means re-adding every sign-in.

**How should secrets be created and the key protected?**

## Options
- A. **Generated on first start by a one-shot `init` service into a `secrets` volume** (master key, DB password via `POSTGRES_PASSWORD_FILE`); the wizard has a required step to download or copy the key, and Settings, System warns until an adult confirms a backup. Bring-your-own files still work (recommended).
- B. Generated on first start, with no wizard step; the guide tells you where the volume is.
- C. Keep hoster-made files as today, but have the guide do it with one copy-paste command.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
A.
