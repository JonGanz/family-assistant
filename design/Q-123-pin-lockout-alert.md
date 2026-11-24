---
id: Q-123
title: PIN lockout notification
status: open
batch: 27
area: security
created: 2026-11-23
resolved:
decisions: []
---

# Q-123: PIN lockout notification

## Context
The PIN lockout says it notifies the adults, but `AuthEvents` is never wired (S-03). Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
Who is notified when a PIN locks out, and how often?

## Options
- A. All adults, once per lockout (recommended)
- B. Configurable, default A: recipients (adults, owner only, none) and a daily cap are settings
- C. Only the account owner, once per lockout
- D. No notification; fix the wording instead

## Answer
<!-- answer below this line -->

