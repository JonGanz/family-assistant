---
id: Q-074
title: Do members pick a named hue instead of a hex colour?
status: open
batch: 19
area: frontend
created: 2026-10-29
resolved:
decisions: []
---

# Q-074: Do members pick a named hue instead of a hex colour?

## Context
A member's `color` is a free-form hex string. Event stripes, chips and task items use it. If a member instead picks a hue name (`red`, `blue`, `yellow`…), each theme supplies its own variant of that hue, so changing theme re-tints every member automatically. Calendars imported from outside (Google, ICS) carry their own hex colours, which are not ours to change.

## Question
**How should member colours work, and what happens to existing hex values?**

## Options
- A. **Members store a hue name from the theme's fixed set. A one-off migration maps each existing hex to the nearest hue. Calendar colours stay hex.** The picker shows the current theme's swatches (recommended).
- B. Hue name by default, but a member may still choose a custom hex (it does not change with the theme).
- C. Keep hex only; just offer theme-derived suggestions in the picker.
- D. **Configurable, default A:** a household setting allows custom hex colours (as in B).
- E. Something else (describe).

## Answer
<!-- answer below this line -->
