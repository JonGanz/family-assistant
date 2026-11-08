---
id: D-120
title: Several meals per slot; template apply replaces or adds per slot
status: accepted
date: 2026-11-08
source_questions: [Q-115]
supersedes: 
superseded_by:
affects: [arch/Meals.md, arch/Frontend.md, arch/Domain.md]
---

# D-120: Several meals per slot; template apply replaces or adds per slot

## Context
Q-115 asked whether a date and slot hold one meal or several. D-112 (templates) had been read as one per slot. Answer: option B, several entries per slot, and a template apply asks per slot whether to replace or add.

## Decision
- A date and slot can hold **any number of entries** (recipes or free text), kept in order (`ord`). Examples: a main and a side, or a children's and an adults' dinner.
- **Template cells** may also hold several items per day and slot, so a template can describe the same plans.
- **Apply:** the preview lists every target slot that already has entries. For each such slot the user picks **Replace** (the existing entries are removed) or **Add** (they stay, and the template's items are appended). A slot holding any free-text entry starts on Add, so "Thanksgiving with cousins" is not clobbered by default; other slots start on Replace. The result is still one change with Undo (D-112).
- Moving an entry onto a filled slot appends it there; **swap** still exchanges two entries.

## Consequences
- `meal_plan_entry` and `meal_template_cell` lose their unique (date|day, slot) index and gain `ord`.
- `TemplateApplyReq` becomes `{start, add_slots: [{date, slot_id}]}`. `TemplatePreview.targets[].existing` becomes a list.
- There is no `slot_taken` error.
- Meal-plan cells, the kiosk Week view and the dashboard `meals` list show every entry.

## Alternatives considered
One entry per slot (Q-115 A); a household setting choosing one or several (Q-115 C).
