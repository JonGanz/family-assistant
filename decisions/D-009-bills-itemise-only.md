---
id: D-009
title: Bills: itemise + create pay task only
status: accepted
date: 2026-10-23
source_questions: [Q-008]
supersedes:
superseded_by:
affects: [arch/Overall.md]
---

# D-009: Bills: itemise + create pay task only

## Context
The user wants only itemisation and tasks, with no budgeting or reports.

## Decision
- A `Bill` has payee, amount, currency, due date, account hint, and a link to the source message.
- Each bill creates a 'Pay <payee>' task that is due before the due date. Marking it paid closes the bill.
- Extraction is deterministic first: biller templates and regex, plus PDF text extraction with the `pdf-extract` crate when the body has no amount. The LLM is the fallback.

## Consequences
- There is no ledger, budget, or export.

## Alternatives considered
Budgeting and trends. Declined by the user.
