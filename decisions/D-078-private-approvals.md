---
id: D-078
title: Approvals are private to the adults who may see the source mail; the dashboard shows only a count per adult
status: accepted
date: 2026-10-26
source_questions: [Q-063]
supersedes: 
superseded_by:
affects: [arch/Ingest.md, arch/Domain.md, arch/Frontend.md]
---

# D-078: Approvals are private to the adults who may see the source mail; the dashboard shows only a count per adult

## Context
A follow-up to the user's Q-063 answer (2026-10-26). They confirmed D-068's reading (no approvals on the dashboard) and added how email-derived items are handled.

## Decision
- Email-derived inbox items (events, bills, tasks, contacts) are **private to the adults who may act on the source mail**: the source's owners, or every adult for a `household` source (the D-063 visibility rule). An adult's inbox lists only those items, and only they may approve, edit or reject them. Granting an adult access to a source (owner, or `household` visibility) is what lets one adult approve for another.
- An item whose source is gone or archived falls back to every adult, so nothing gets stuck.
- **The dashboard may show, at most, that an adult has pending approvals:** a count badge on that adult in the members strip, with no titles, senders or kinds. A household setting `dashboard_pending_approvals` (configurable, default on) turns it off.
- The actual approval always happens in the app, signed in as that adult (D-068).

## Consequences
- Refines D-068 (the dashboard showed nothing from email) and D-063 (visibility now also scopes approvals).
- `approval_needed` notifications (D-061) go only to the adults who may approve the item.
- An item from a `household` source counts in every adult's badge until one of them acts on it.
- Needs a household column, set from Settings, Household, and per-adult counts in the dashboard data (D-064 allowlist unchanged: the counts ride on the existing dashboard call).

## Alternatives considered
- No indicator at all (D-068 as first written): adults would not know to open the app.
- Any adult approves anything (M3 behaviour): rejected, approvals are private to the source's adults.
