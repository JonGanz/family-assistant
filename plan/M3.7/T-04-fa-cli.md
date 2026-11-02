---
id: T-04
title: "tools/fa.py as a CLI with keyword-flag subcommands (P-06)"
milestone: M3.7
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["tools/fa.py", "tools/test_fa.py", ".claude/skills/process-answers/SKILL.md", "README.md"]
decisions: [D-066]
---

# T-04: `tools/fa.py` as a CLI with keyword-flag subcommands (P-06)

You are a sub-agent implementing one task of milestone M3.7 (review follow-ups) of the Family Assistant project. This task is pure Python and docs: no Rust, no web.

## Read first
- `plan/_rules-common.md` and `plan/M3.7/_rules.md`
- `plan/M3.7.md`: *P-06 fa.py CLI* (the subcommand contract)
- `CLAUDE.md` (front-matter rules, workflows 1, 2, 5, 6), `tools/fa.py`, `tools/test_fa.py`, `.claude/skills/process-answers/SKILL.md`, `design/_index.md` (the question template and index layout), `decisions/_index.md`, `log/_index.md`, `plan/human-steps.md`, one plan (`plan/M3.6.md`) for the task-status table layout
- The finding: `fa.py` is a library and the skill makes agents write `python3 -c` one-liners whose positional arguments silently corrupt files (P-06); index `updated:` dates go stale because writers don't stamp them (P-03).

## You own
`tools/fa.py`, `tools/test_fa.py`, the `process-answers` skill, and the README's tooling rows. Nothing else; do not run the writers against the real `design/`, `decisions/` or `log/` trees (use temp dirs in tests).

## Work
1. `argparse` subcommands exactly as the plan lists, keyword flags only, `@path` for long text, `--n`/`next-id` from the highest existing number across `design/`, `design/resolved/` and `decisions/`. The library functions remain importable and keep their signatures (the subcommands call them).
2. Every writer stamps `updated: <today>` in the index it touches (design, decisions, log); `rebuild_design_index` too.
3. `log --topics --text`: creates `log/<today>.md` with front-matter `date`, `topics: []` when missing, appends a `## ` section otherwise (topics merged into the front-matter list), and adds the `log/_index.md` line if new.
4. `status`: writes `STATUS.md` (repo root) from the sources named in the plan; wire `python3 tools/fa.py check` to also report when `STATUS.md` is older than any of its sources only if that is simple, otherwise skip and say so.
5. Tests in `tools/test_fa.py` for every subcommand, each in a temp tree (`subprocess.run([sys.executable, fa_path, ...], cwd=tmp)`), including: `new-q` then `check` passes; `new-q` with positional arguments is rejected by argparse; `@file` text; `resolve` with a stub git repo (`git init` in the temp dir); `log` creates then appends; `status` output lists an open question and an open human step.
6. Rewrite the `process-answers` skill to use the subcommands (no more `python3 -c`). Keep its numbered steps.
7. README: the Tooling / commands rows for `fa.py` (find the existing mention).

## Acceptance
- [ ] `python3 tools/test_fa.py` and `python3 tools/fa.py check` pass; `python3 tools/fa.py --help` lists every subcommand
- [ ] `python3 tools/fa.py status` run once in the real repo and the resulting `STATUS.md` looks right (you may keep it; it is a generated file, add it to the README as such)
- [ ] `scripts/ci-local.sh` steps that touch Python pass (lines 8 to 10)
- [ ] report: subcommand list, what `status` includes, README rows changed
