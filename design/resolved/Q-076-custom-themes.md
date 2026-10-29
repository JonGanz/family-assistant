---
id: Q-076
title: Can a household add its own themes?
status: resolved
batch: 19
area: frontend
created: 2026-10-29
resolved: 2026-10-29
decisions: [D-079]
---

# Q-076: Can a household add its own themes?

## Context
Each theme is a small set of values: neutral levels and the named hues. Shipped themes are checked against contrast rules (text on each surface level, hues as accents) when the app is built. A user-defined theme would need the same check at the time it is saved.

## Question
**Can a household define its own themes?**

## Options
- A. **Built-in themes only for now. The theme format is plain data, so custom themes can be added later** (recommended).
- B. Adults can paste or edit a theme in Settings. The app checks contrast when saving and warns on failures.
- C. **Configurable, default A:** a household setting enables B.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option A.
