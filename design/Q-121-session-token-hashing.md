---
id: Q-121
title: Hash session tokens in the database
status: open
batch: 27
area: security
created: 2026-11-23
resolved:
decisions: []
---

# Q-121: Hash session tokens in the database

## Context
The session cookie value is the `session.id` primary key in plaintext (`fa-auth/src/login.rs:172`), so a database or backup read gives live sessions (S-01). Hashing logs every session out once on upgrade. Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
Store only a hash of the session token?

## Options
- A. Yes: store a hash; everyone signs in again once after the upgrade (recommended)
- B. Yes, and keep old plaintext sessions valid for a grace period (more code)
- C. No; accept the risk because backups are the household's own

## Answer
<!-- answer below this line -->

