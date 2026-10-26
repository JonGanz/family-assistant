---
id: Q-059
title: The labelling view: what you label and how messages are picked
status: resolved
batch: 16
area: ingest
created: 2026-10-25
resolved: 2026-10-25
decisions: [D-062]
---

# Q-059: The labelling view: what you label and how messages are picked

## Question
D-028: a labelling view builds a ~200-message sample for `fa-eval`, so that M4 can pick and tune the AI models. Two choices affect how much effort it takes:
1. **What is labelled per message:** only the category, or every email decision point (category, time-sensitive, member links, is-bill)?
2. **How messages are picked:** you browse and label what you like, or the system suggests a balanced mix (stratified by category, and by rule-decided vs fell-to-default) so the sample isn't all newsletters.

The view is phone-friendly: one message at a time, with the current guess pre-filled, so most messages are one tap.

**What should the defaults be?**

## Options
- A. Category only, browse freely.
- B. **All four decision points with the current guess pre-filled, and a suggested balanced queue (you can still label any message from the message view)** (recommended). About 200 messages at one or two taps each.
- C. All four decision points, browse freely.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option B.
