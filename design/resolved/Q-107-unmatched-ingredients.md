---
id: Q-107
title: Ingredient lines the rules cannot match
status: resolved
batch: 24
area: ingest
created: 2026-11-07
resolved: 2026-11-08
decisions: [D-113]
---

# Q-107: Ingredient lines the rules cannot match

## Context
Deterministic first: the line parser, aliases, singular/plural and trigram similarity match most lines to the catalogue. Some will not match ("1 (15-oz) can fire-roasted tomatoes, drained"). The local LLM can propose a match or a new ingredient as structured JSON.

## Question
What happens to an unmatched line?

## Options
- A. Rules first; unmatched lines go to the LLM in the background, which proposes a match or a new ingredient; an adult confirms in the recipe editor; until then the line goes on the list as its text (suggested)
- B. Rules only; unmatched lines go on the list as text and an adult can link them by hand
- C. LLM proposals apply without confirmation
- D. Configurable in Settings → Meals (off / suggest / apply), default suggest

## Answer
<!-- answer below this line -->
Option A.
