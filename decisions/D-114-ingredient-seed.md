---
id: D-114
title: Ship a hand-written ingredient seed
status: accepted
date: 2026-11-08
source_questions: [Q-106]
supersedes: 
superseded_by:
affects: [arch/Meals.md]
---

# D-114: Ship a hand-written ingredient seed

## Context
Q-106 asked what a new household starts with. Answer: option A, within licence requirements. R-007: Open Food Facts is ODbL (share-alike and attribution), USDA FDC is reported CC0 but only from a search snippet.

## Decision
The repo ships a hand-written seed of about 300 common ingredients (canonical name, aliases including plurals, aisle, default unit; staples off), written for this project under its AGPL licence. No third-party dataset is copied. It is loaded once per household at creation (and for existing households by a migration-time job), and is fully editable.

## Consequences
A data file in `crates/fa-meals/seed/` with a test that checks uniqueness and alias collisions. Densities are not needed (D-113).

## Alternatives considered
An open-dataset subset (Q-106 B; licence duties), an empty catalogue (Q-106 C).
