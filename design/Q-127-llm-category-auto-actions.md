---
id: Q-127
title: LLM-set mail categories and auto-actions
status: open
batch: 27
area: security
created: 2026-11-23
resolved:
decisions: []
---

# Q-127: LLM-set mail categories and auto-actions

## Context
Mail bodies reach the LLM undelimited, its confidence is self-reported, and auto-actions follow a category with no human step (`fa-mail/src/process/llm.rs:337`) (S-05). Related: G4-03. Source: the 2026-11-23 review (IMPROVEMENTS).

## Question
Should an LLM-set category trigger auto-actions?

## Options
- A. Configurable, default: LLM categories never auto-act; they propose and a person approves (recommended)
- B. Configurable, default: auto-act only above a confidence threshold and for a safe action list
- C. Keep as is

## Answer
<!-- answer below this line -->

