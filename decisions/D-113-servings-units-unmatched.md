---
id: D-113
title: Servings scale the list; units convert within a dimension; the LLM proposes matches for unmatched lines
status: accepted
date: 2026-11-08
source_questions: [Q-104, Q-105, Q-107]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/AI.md]
---

# D-113: Servings scale the list; units convert within a dimension; the LLM proposes matches for unmatched lines

## Context
D-015 requires normalisation (units, singular and plural) and merging. Answers: Q-104 A, Q-105 A, Q-107 A. R-007 informs the method.

## Decision
- **Servings (Q-104 A):** each entry has servings, defaulting to the recipe's; generated quantities scale by entry servings / recipe servings.
- **Units (Q-105 A):** convert within a dimension (volume in ml, weight in g, count) and show totals in the household's units (`household.units`), rounded to kitchen sizes. Never volume↔weight: such lines stay separate quantities on one item.
- **Unmatched lines (Q-107 A):** rules first (parser, cleaning, singularisation, alias, trigram). An unmatched line goes on the list as its text; in the background the `ingredient.llm` job sends it with the five nearest catalogue entries to the LLM, which proposes a match or a new ingredient as structured JSON. An adult confirms in the recipe editor; a confirmed or manual pick is saved as an alias so the next import matches by rules. The LLM never creates catalogue rows itself. With no LLM endpoint the job does nothing.

## Consequences
`fa-meals` normaliser per R-007 (`pluralizer`, `strsim`/`pg_trgm`, a hand-written unit table).

## Alternatives considered
Household-size servings (Q-104 B), densities (Q-105 B), rules only or auto-apply (Q-107 B, C).
