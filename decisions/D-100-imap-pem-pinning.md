---
id: D-100
title: Per-source PEM for IMAP TLS; plain IMAP stays off
status: accepted
date: 2026-11-07
source_questions: [Q-080]
supersedes: 
superseded_by:
affects: [arch/Ingest.md]
---

# D-100: Per-source PEM for IMAP TLS; plain IMAP stays off

## Context
FA_ALLOW_PLAIN_IMAP and FA_IMAP_TLS_INSECURE are process-wide test switches, so a LAN IMAP server with a private CA cannot be used safely.

## Decision
An IMAP mail source takes an optional PEM (a server certificate or a CA bundle) in Settings → Mail; when set, TLS verifies against it instead of the system roots. Plain IMAP stays off for every source. The two process-wide switches remain tests-only.

## Consequences
Needs an encrypted column for the PEM on the mail source and a rustls verifier built per source. LARGER item for the next milestone.

## Alternatives considered
B (plain IMAP to private hosts behind a toggle), C (process-wide switches only).
