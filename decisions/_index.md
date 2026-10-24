---
title: Decisions Index
updated: 2026-10-23
---

# Decisions

Each record is `D-NNN-slug.md`. Superseded decisions stay here and are marked with `status: superseded`.

| ID | Title | Status | Source Qs | Affects |
|---|---|---|---|---|
| [D-001](D-001-local-first-deterministic-first.md) | Local-first, deterministic-first processing | accepted | PROJECT.md | Overall |
| [D-002](D-002-hosting-split.md) | Split hosting: always-on server + on-demand GPU node; LLM is an enhancement | accepted | Q-001 | arch/Overall.md |
| [D-003](D-003-rust-vue-stack.md) | Backend in Rust (Axum/Tokio/SQLx); frontend Vue 3 + TypeScript | accepted | Q-002 | arch/Overall.md |
| [D-004](D-004-config-driven-household.md) | Household data is configuration, never in the repo; multi-family capable | accepted | Q-003 | arch/Overall.md |
| [D-005](D-005-postgres-nas-backups.md) | PostgreSQL 16 + pgvector; nightly backups to NAS | accepted | Q-004 | arch/Overall.md |
| [D-006](D-006-auth-rbac-dashboard.md) | Per-user logins with role-based access; shared dashboard with per-action member selection | accepted | Q-005 | arch/Overall.md, arch/Frontend.md (future) |
| [D-007](D-007-email-accounts-model.md) | Multiple Gmail accounts mapped to members; read-only by default with opt-in actions | accepted | Q-006 | arch/Overall.md, arch/Ingest.md (future) |
| [D-008](D-008-calendar-sources-routing.md) | Multiple calendar sources; confidence-gated auto-add; configurable + learned target routing | accepted | Q-007 | arch/Overall.md, arch/Ingest.md (future) |
| [D-009](D-009-bills-itemise-only.md) | Bills: itemise + create pay task only | accepted | Q-008 | arch/Overall.md |
| [D-010](D-010-tailscale-ntfy.md) | Remote access via Tailscale; Android notifications via self-hosted ntfy | accepted | Q-009 | arch/Overall.md |
