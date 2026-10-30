---
title: Plans Index
updated: 2026-10-29
---

# Plans

There is one plan per milestone (`M<n>.md`). Its sub-agent task briefs are in `M<n>/T-NN-slug.md`.

**What is waiting on a person:** [human-steps.md](human-steps.md). Open now: H-12 (GHCR publishing and the first `v*` tag), H-13 (timed fresh install on the real server), H-14 (Tailscale sidecar from a phone), then H-02 to H-11 redone on that install.

| Milestone | Plan | Status | Tasks |
|---|---|---|---|
| M0 Skeleton | [M0.md](M0.md) | code-complete, awaiting human steps H-02 to H-04 | [M0/](M0/): T-01 to T-12, plus human steps H-01 to H-04 |
| M1 Dashboard core | [M1.md](M1.md) | code-complete, awaiting human steps H-05 to H-07 | [M1/](M1/): T-01 to T-12, plus human steps H-05 to H-07 |
| M2 Tasks | [M2.md](M2.md) | code-complete, awaiting human steps H-08, H-09 | [M2/](M2/): T-01 to T-14 (+ `_rules.md`), plus human steps H-08, H-09 |
| M3 Ingest | [M3.md](M3.md) | code-complete, awaiting human steps H-10, H-11 | [M3/](M3/): T-01 to T-15 (+ `_rules.md`), plus human steps H-10, H-11 |
| M3.5 Onboarding and shakedown | [M3.5.md](M3.5.md) | code-complete, awaiting human steps H-12 to H-14, then the shakedown (H-02 to H-11 redone on the new install, U-16, a week of inbox numbers) | [M3.5/](M3.5/): T-01 to T-10 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)), plus human steps H-12 to H-14 and the shakedown |
| M3.6 Themes | [M3.6.md](M3.6.md) | code-complete; e2e, clippy and tests green, compose-smoke pending (D-079 to D-081) | [M3.6/](M3.6/): T-01 to T-08 (+ `_rules.md`, on top of [`_rules-common.md`](_rules-common.md)); T-09 integration is the main session |

## Reviews
Time-boxed review sessions (CLAUDE.md workflow 7). Each file has findings tables with IDs, sizes and status.

| Date | Review | Open items |
|---|---|---|
| 2026-10-26 | [Repository review: process, code, direction, UI](reviews/2026-10-26-repo-review.md) | M3.5 closed its share. Left for M4: C-10, C-13, C-14, P-06, P-07, the second code review. Shakedown: R-01 to R-04, U-16 |
