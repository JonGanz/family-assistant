---
id: D-092
title: The dashboard has a bouncing-icon screensaver after 15 idle minutes
status: accepted
date: 2026-10-30
source_questions: []
supersedes: 
superseded_by:
affects: [arch/Frontend.md]
---

# D-092: Dashboard screensaver

## Context
Direct user instruction (2026-10-30, no question): a screensaver showing a black screen with the Family Assistant icon bouncing like an old DVD player logo, starting after 15 minutes without interaction, dismissed by tapping.

## Decision
- The dashboard page shows a full-screen black overlay with the app icon moving diagonally and bouncing off the screen edges. The data underneath keeps updating, and the watchdog and connection status are unaffected.
- Any pointer, touch, key or wheel event restarts the idle countdown. While the screensaver shows, a tap (a `click` on the overlay, so the tap does not also press a button beneath) dismisses it and restarts the countdown.
- **Configurable, default 15:** the device setting `screensaver_minutes` (0 turns it off, at most 240; absent in stored settings means 15) is edited in Settings, Devices, Edit layout. This follows the standing rule that behaviour likely to vary is editable (the user's instruction fixed the default, not the configurability).
