---
id: D-111
title: Editable meal slots; only Dinner shown by default
status: accepted
date: 2026-11-08
source_questions: [Q-102]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Domain.md]
---

# D-111: Editable meal slots; only Dinner shown by default

## Context
D-015 made the slots configurable; Domain.md had them as a fixed enum. Answer: option A, but default only to Dinner.

## Decision
Slots are an editable per-household list in Settings → Meals: name, order, a `"HH:MM"` local time, and shown on or off. A new household gets Breakfast 07:30, Lunch 12:00, Dinner 18:00 and Snack 15:30, with **only Dinner shown**; the others are there to switch on. Adults can add, rename and reorder slots.

## Consequences
`meal_slot` table replaces the enum. Hidden slots keep their entries but are not drawn.

## Alternatives considered
Fixed slots (Q-102 B, C); creating only a Dinner slot (rejected: switching a slot on is easier than creating one, and the answer's wording is about the default view).
