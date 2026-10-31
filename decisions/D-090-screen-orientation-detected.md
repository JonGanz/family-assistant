---
id: D-090
title: The dashboard's orientation is detected from the screen, not configured
status: accepted
date: 2026-10-30
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-090: Orientation is detected from the screen

## Context
Direct user instruction (2026-10-30, no question), refining D-082: the menu follows the screen's shorter edge, and the per-device `orientation` setting is removed in favour of the detected screen dimensions.

## Decision
- `DashboardSettings` no longer has `orientation`. The dashboard measures its own size: **portrait** when taller than wide, else **landscape**. It re-measures when the size changes.
- That one value picks the panel layout (two stacks in landscape, one in portrait) and the menu edge (top in portrait, left in landscape). The menu buttons have the same size in both.
- A stored `orientation` key on an existing device is ignored (the server does not reject unknown keys), so no migration is needed.
- The layout editor has no orientation field. Its live preview has its own "Preview as" shape choice, which only sizes the preview.
