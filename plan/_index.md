---
title: Plans Index
updated: 2026-11-07
---

# Plans

There is one plan per milestone (`M<n>.md`). Its sub-agent task briefs are in `M<n>/T-NN-slug.md`.

**What is waiting on a person:** [human-steps.md](human-steps.md). Open now: H-15 to H-17 (M4), H-12 (GHCR publishing and the first `v*` tag), H-13 (timed fresh install on the real server), H-14 (Tailscale sidecar from a phone), then H-02 to H-11 redone on that install.

| Milestone | Plan | Status | Tasks |
|---|---|---|---|
| M0 Skeleton | [M0.md](M0.md) | code-complete, awaiting human steps H-02 to H-04 | [M0/](M0/): T-01 to T-12, plus human steps H-01 to H-04 |
| M1 Dashboard core | [M1.md](M1.md) | code-complete, awaiting human steps H-05 to H-07 | [M1/](M1/): T-01 to T-12, plus human steps H-05 to H-07 |
| M2 Tasks | [M2.md](M2.md) | code-complete, awaiting human steps H-08, H-09 | [M2/](M2/): T-01 to T-14 (+ `_rules.md`), plus human steps H-08, H-09 |
| M3 Ingest | [M3.md](M3.md) | code-complete, awaiting human steps H-10, H-11 | [M3/](M3/): T-01 to T-15 (+ `_rules.md`), plus human steps H-10, H-11 |
| M3.5 Onboarding and shakedown | [M3.5.md](M3.5.md) | code-complete, awaiting human steps H-12 to H-14, then the shakedown (H-02 to H-11 redone on the new install, U-16, a week of inbox numbers) | [M3.5/](M3.5/): T-01 to T-10 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)), plus human steps H-12 to H-14 and the shakedown |
| M3.6 Themes | [M3.6.md](M3.6.md) | code-complete; e2e, clippy and tests green, compose-smoke pending (D-079 to D-081) | [M3.6/](M3.6/): T-01 to T-08 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)); T-09 integration is the main session |
| M3.7 Review follow-ups | [M3.7.md](M3.7.md) | running (C-10, C-13, C-14, P-06, P-07, second code review) | [M3.7/](M3.7/): T-01 to T-07 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)) |
| M4 UI revamp and LLM extraction slice | [M4.md](M4.md) | done 2026-11-07 (code-complete; H-15 to H-17 open) | [M4/](M4/): T-01 to T-23 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)), plus human steps H-15 to H-17 |
| M5 Recipes, meal plan, grocery list | [M5.md](M5.md) | planned 2026-11-08 (D-108 to D-122) | [M5/](M5/): T-01 to T-13 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)), waves 1 to 6 |

## Reviews
Time-boxed review sessions (CLAUDE.md workflow 7). Each file has findings tables with IDs, sizes and status.

| Date | Review | Open items |
|---|---|---|
| 2026-11-02 | [Code review 2: fa-mail internals, ICS and CalDAV, tasks, notify](reviews/2026-11-02-code-review-2.md) | QUICK groups fixed in M3.7 wave 2 (T-08 to T-13). LARGER: C2-02, C2-05, C2-08, C2-09, C2-15, C2-19, C2-21, C2-30 for M4. Questions Q-078 to Q-082 |
| 2026-10-26 | [Repository review: process, code, direction, UI](reviews/2026-10-26-repo-review.md) | M3.5 closed its share. Left for M4: C-10, C-13, C-14, P-06, P-07, the second code review. Shakedown: R-01 to R-04, U-16 |
