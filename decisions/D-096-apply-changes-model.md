---
id: D-096
title: Single controls apply instantly with Undo; multi-field forms use a Save/Discard bar
status: accepted
date: 2026-11-04
source_questions: [Q-086]
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-096: Single controls apply instantly with Undo; multi-field forms use a Save/Discard bar

## Context
Pages apply changes in different ways today: some save on change, some need a button, some have both. Q-086.

## Decision
One rule everywhere outside the dashboard:
- **Single controls apply instantly:** a switch, a single choice (radio, segmented button, select that stands alone) or a checkbox in a list saves on change and shows a snackbar with **Undo** (about 5 s). A failure reverts the control and shows the error in the snackbar.
- **Multi-field forms use a Save/Discard bar:** the form tracks dirty state; a sticky bar appears while it is dirty (bottom on phones, top of the card on wide screens); leaving the page with unsaved changes asks first; success shows a snackbar.
- Creating something (a new task, a new source) is a form in a dialog or bottom sheet with its own Save, never instant.
- Destructive actions confirm in a dialog unless they offer Undo.

## Consequences
- The library provides a `useInstantSave` composable (optimistic update plus Undo) and a `useFormDraft` composable (dirty tracking, Save/Discard bar, leave guard); views use those instead of their own logic.

## Alternatives considered
- Configurable with a Save-bar default; Save bar everywhere; autosave per field. Not chosen.
