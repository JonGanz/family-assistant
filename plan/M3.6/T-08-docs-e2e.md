---
id: T-08
title: "Docs, e2e and fixtures"
milestone: M3.6
wave: 4
depends_on: [T-02, T-03, T-04, T-05, T-06, T-07]
agent: {model: sonnet, effort: medium}
status: done
owns: ["README.md", "e2e/**", "docs/**", "apps/web/e2e/**", "plan/M3.6.md", "plan/M3.6/**", "arch/Frontend.md", "arch/Domain.md", "log/**"]
decisions: [D-079, D-080, D-081]
---

# T-08: Docs, e2e and fixtures

You are a sub-agent implementing one task of milestone M3.6 (themes) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md`, `plan/M3.6/_rules.md`, `plan/M3.6.md`
- the other tasks' reports (added below when done); `e2e/setup.spec.ts` and the specs that set a member colour

## You own
The paths in the front matter. Not the indexes (`plan/_index.md`, `log/_index.md`): the main session does those.

## Work
1. **e2e:** update every spec and fixture that sets or reads a member colour to hue names (`e2e/setup.spec.ts` and any others), and add one spec: pick a theme in Settings → Appearance, reload, and check the choice stuck and `data-theme-mode` matches. Run it only if the stack is up; otherwise say so.
2. **README:** a short "Themes" note for users (where to change it, per device, kiosk setting in Devices) and for contributors (how to add a family: the file format, `FAMILIES`, the contrast test). Update the roadmap row. Verify any command you write by running it.
3. **Arch docs:** read `arch/Frontend.md` §3d and `arch/Domain.md` against what was built and correct anything that differs (the engine's real export names, the stored key). If the build deviated from a *decision*, do not edit the decision: report it.
4. `plan/M3.6.md`: set the status, add `## Outcome` with known gaps (from the reports), and set every task's `status` to what it is.

## Acceptance
- [ ] `python3 tools/fa.py check` passes
- [ ] README steps you wrote were run
- [ ] report: deviations and open items for the main session's log
