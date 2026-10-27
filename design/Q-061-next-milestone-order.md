---
id: Q-061
title: What comes next: real-hardware shakedown, M4 AI, or M5 food?
status: open
batch: 17
area: infra
created: 2026-10-26
resolved:
decisions: []
---

# Q-061: What comes next: real-hardware shakedown, M4 AI, or M5 food?

## Question
The 2026-10-26 review ([plan/reviews/2026-10-26-repo-review.md](../plan/reviews/2026-10-26-repo-review.md), section 3) found that M0–M3 are code-complete but every human step H-02..H-11 is still open, so nothing has run on real data and the Echo Show (the main interface) is unproven. D-023 says each milestone ends with a demo on real hardware. Meanwhile M5 (food) is deterministic daily value that needs no GPU, and the M3 defaults send every extracted event, task and contact to the approval inbox, whose real volume is unknown.

**Which order should the next work take?**

## Options
- A. **M3.5 shakedown first** (recommended): deploy on the real server, do H-02..H-11 (one real mail source, one Google and one CalDAV calendar, the Echo Show via D-064, ntfy on phones), record approval-inbox volume for a week, fix the M3 known gaps; then M4.
- B. M4 (local AI) next, as the roadmap says (D-023, D-050); the shakedown happens when it happens.
- C. M5 (food: recipes, meal plan, grocery) next, in parallel with the shakedown; M4 after both.
- D. Something else (describe).

## Answer
<!-- answer below this line -->
A.
