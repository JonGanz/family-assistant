---
id: Q-017
title: Configuration source of truth
status: resolved
batch: 2
area: domain
created: 2026-10-23
resolved: 2026-10-23
decisions: [D-018]
---

# Q-017: Configuration source of truth

## Question
D-004 makes household data configuration. Should that configuration be **declarative files** (a TOML/YAML file kept outside the repo, git-versionable privately, and loaded at start), **managed in the UI** (stored in the DB and edited through settings screens), or a **hybrid** (files to bootstrap, then the UI owns it, with export and import)?

## Options
- A: hybrid: file bootstrap/import-export, UI is the source of truth after (recommended)
- B: files only
- C: UI only

## Answer
<!-- answer below this line -->
A.
