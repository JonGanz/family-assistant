---
id: T-07
title: "Skills plan-milestone, run-wave, close-milestone (P-07)"
milestone: M3.7
wave: 2
depends_on: [T-04]
agent: {model: sonnet, effort: high}
status: done
owns: [".claude/skills/plan-milestone/**", ".claude/skills/run-wave/**", ".claude/skills/close-milestone/**", "CLAUDE.md"]
decisions: [D-066]
---

# T-07: Skills `plan-milestone`, `run-wave`, `close-milestone` (P-07)

You are a sub-agent writing three Claude Code skills for the Family Assistant project. This task is docs only.

## Read first
- `CLAUDE.md` in full (workflows 5 to 8, "Implementation via sub-agents", "Git", "README", "Self-learn"): the skills must encode these rules, not restate them loosely
- `.claude/skills/process-answers/SKILL.md` (format: front-matter `name`, `description`, then numbered steps; it was just rewritten by T-04 to use the `fa.py` CLI, so read it after T-04 is done)
- `tools/fa.py --help` (the subcommands the skills can call: `log`, `status`, `check`, `next-id`)
- Two worked milestones as the model: `plan/M3.6.md` with `plan/M3.6/_rules.md` and its briefs; `plan/M3.7.md` with its briefs; `plan/_rules-common.md`; `plan/human-steps.md`; `plan/_index.md`
- The finding (P-07): every milestone repeats plan → rules → briefs → cross-check → waves → integrate → close, and only `process-answers` is a skill.

## You own
The three skill directories and **one** small edit in `CLAUDE.md`: a line in "Workflows" (or next to the skill mention in workflow 2) that names the three skills and when to use them. Nothing else in CLAUDE.md.

## Work
Each skill is `.claude/skills/<name>/SKILL.md` with front-matter `name` and `description` (the description says when to use it, in one sentence, as `process-answers` does) and numbered, checkable steps. Reference files by path; quote commands in code blocks.
1. **`plan-milestone`**: from a scope (decisions, review findings or a user request) to `plan/M<n>.md` (front-matter, Why, Done means, Contracts fixed up front with the waves table and wire formats, Out of scope, Task status), `plan/M<n>/_rules.md` (deltas only), one brief per task (front-matter per CLAUDE.md incl. `decisions:`, Read first, You own, Work, Acceptance), the cross-check list from CLAUDE.md (every column, table or dependency owned by an earlier or same-wave task; one task for `fa-domain` + `fa-store` changes; every date field's wire format; what writes every new column), `plan/_index.md` row, `fa.py check`, log.
2. **`run-wave`**: disk check and cleanup (the one disk rule), launch the wave's briefs as parallel sub-agents (Sonnet 5.5, effort from the brief's front-matter) with the brief file as the prompt, strict ownership, then integrate: build, test, regenerate `apps/web/src/gen` when DTOs changed (the README command), fix small issues, set task `status` and `## Outcome` in each brief, log with `fa.py log`, commit with a `<- ` one-line message once the CI-equivalent checks pass.
3. **`close-milestone`**: the CLAUDE.md workflow 8 checklist as steps with the exact commands (`scripts/ci-local.sh`, `scripts/e2e.sh`, `scripts/compose-smoke.sh`, `docker volume ls -f dangling=true`), every task `done`, `## Outcome` with known gaps, README roadmap verified by running the steps, `plan/_index.md` and `plan/human-steps.md` updated, `fa.py status`, log, commit.

## Acceptance
- [ ] the three `SKILL.md` files exist and each has `name` and `description` front-matter
- [ ] `python3 tools/fa.py check` passes
- [ ] report: the CLAUDE.md line added, and anything in CLAUDE.md that you found contradicting the plans you read (report, do not fix)
