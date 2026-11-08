---
id: Q-098
title: "Recipe import: which ways in"
status: resolved
batch: 24
area: ingest
created: 2026-11-07
resolved: 2026-11-08
decisions: [D-109]
---

# Q-098: Recipe import: which ways in

## Context
M4 built `POST /ai/recipes/extract`: schema.org JSON-LD first, the LLM only for pages without it (AI.md §9). Domain.md also lists `microdata` as an import method. M5 stores recipes, so it needs the ways a recipe gets in.

## Question
Which import paths should M5 offer?

## Options
- A. URL import (JSON-LD, then schema.org microdata, then the LLM), pasted page text through the LLM, and a manual form; each result opens as an editable draft before saving (suggested)
- B. A, plus a photo of a cookbook page (local OCR, then the LLM)
- C. URL (JSON-LD and microdata only) and the manual form; no LLM in recipe import
- D. Configurable: each path is a toggle in Settings → Meals; default A

## Answer
<!-- answer below this line -->
Option A.
