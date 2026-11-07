---
id: D-108
title: M5 (food) starts before M4.1
status: accepted
date: 2026-11-07
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Overall.md]
---

# D-108: M5 (food) starts before M4.1

## Context
The roadmap (D-023, D-066, D-094) put M4.1 (System One sidecar, kNN, backend registry, GPU agent, full eval) after M4 and before M5. On 2026-11-07, after M4 closed, the user asked to start M5. This record comes from that instruction, not from a question (CLAUDE.md, D-064 model).

## Decision
M5 (recipes, meal plan, living grocery list; D-015, D-021) is the next milestone. M4.1 follows it. M5 uses the M4 LLM slice (fa-llm, the recipe-extraction contract) as it is and needs nothing from M4.1.

## Consequences
The roadmap tables in README.md and arch/Overall.md list M5 before M4.1. arch/Meals.md is written now. Whether M5 also takes the M4 follow-ups is Q-114.

## Alternatives considered
M4.1 first, as the roadmap had it. Not chosen by the user.
