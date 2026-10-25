---
id: Q-038
title: Pool tasks: claim first, or just do it
status: resolved
batch: 14
area: domain
created: 2026-10-24
resolved: 2026-10-25
decisions: [D-040]
---

# Q-038: Pool tasks: claim first, or just do it

## Question
D-013's `pool` strategy lets anyone in a set of members take a task.

**How does a member take a pool task?**

## Options
- A. **Just do it:** anyone in the pool checks it off. No claiming.
- B. **Claim first:** a member claims the task (it then shows as theirs) and completes it later. A claim can be released.
- C. **Configurable per list, default A with optional claiming** (recommended): checking off works without a claim. A member *can* claim to signal "I'm on it". A claim expires after a configurable time (default end of day) if not done.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
Option C. 
