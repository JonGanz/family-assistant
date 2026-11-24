---
id: Q-122
title: Master key download over plain HTTP
status: open
batch: 27
area: security
created: 2026-11-23
resolved:
decisions: []
---

# Q-122: Master key download over plain HTTP

## Context
The master key download does not require a secure connection (`fa-api/src/system.rs:63`) (S-02). D-072 keeps the app on plain HTTP on the home network. Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
What should the key download do on a non-secure connection?

## Options
- A. Refuse unless the connection is HTTPS or loopback; the page explains how to reach it (recommended)
- B. Warn, then allow
- C. Configurable, default A: a setting lets an operator allow it on a trusted LAN

## Answer
<!-- answer below this line -->

