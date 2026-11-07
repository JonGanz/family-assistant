---
id: Q-080
title: "Self-hosted IMAP on the LAN: certificate pinning and plain IMAP"
status: open
batch: 21
area: ingest
created: 2026-11-02
resolved:
decisions: []
---

# Q-080: Self-hosted IMAP on the LAN: certificate pinning and plain IMAP

## Question
FA_ALLOW_PLAIN_IMAP and FA_IMAP_TLS_INSECURE are process-wide, tests-only switches, so a LAN Dovecot with a private CA cannot be used safely. Should a mail source be able to pin a certificate or CA, and should plain IMAP be allowed to private hosts?

## Options
- A. Optional PEM (certificate or CA) per source; plain IMAP off (suggested default)
- B. Optional PEM per source; plain IMAP allowed to private hosts behind a per-source toggle, default off
- C. Keep the process-wide switches only

## Answer
<!-- answer below this line -->
Option A.
