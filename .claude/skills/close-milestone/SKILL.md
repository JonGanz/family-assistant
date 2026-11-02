---
name: close-milestone
description: Run the CLAUDE.md workflow 8 closing checklist for a milestone (full CI, e2e, compose smoke, dangling volumes, outcome, README, indexes, log, commit). Use when the user says to close or finish a milestone, or when the last wave is integrated.
---

# Close a milestone

Run from the repo root in the main session. Never mark a milestone closed on the strength of sub-agent reports: run the commands yourself and quote numbers only from their output.

1. **Every task `done`.** Check each brief's `status` and the plan's *Task status* table with `python3 tools/fa.py status`. A task that is not `done` means the milestone is not closing: finish it with `run-wave`, or move it to the next plan and say so in `## Outcome`.
2. **Disk.** `df -h /` (stop below 6 GB), delete `target/debug/incremental`, `docker builder prune -f` (never prune volumes).
3. **Full CI, locally and against a fresh Postgres:**
   ```bash
   scripts/ci-local.sh
   ```
   It runs `python3 tools/fa.py check`, `tools/test_fa.py`, fmt, clippy, the Rust tests (fresh container, `SQLX_OFFLINE=true`, GreenMail) and the web checks. If you edited a Dockerfile or added a `COPY`, confirm every source exists. When web code changed, `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` pass in `apps/web`.
4. **End to end** (you run it yourself, even if every unit test passed: a view that never rendered its data showed up only here in M1):
   ```bash
   scripts/e2e.sh
   ```
5. **Install smoke:**
   ```bash
   scripts/compose-smoke.sh
   ```
6. **No leaked volumes:**
   ```bash
   docker volume ls -f dangling=true
   ```
   The result must be empty. If a volume predates the milestone (check the creation date with `docker volume inspect`), say so in `## Outcome` and leave it; never prune volumes without asking. If the milestone leaked one, find the container that was removed without `-v` and fix the script.
7. **README.** Update it in the same change if setup, commands, env vars, ports, layout, deployment or milestone status changed (the Roadmap table always changes). Then **run the README steps** you changed or that the milestone touched (install, commands, the regenerate-types line) and fix what does not work. "Verified" means executed.
8. **Write `## Outcome` in `plan/M<n>.md`:** what shipped, the exact commands run and their results (counts from the output), **known gaps**, and the checklist items not run with the reason. Set the plan's `status` and `updated`. Make sure every brief has its own `## Outcome (T-NN report)`.
9. **Indexes and human steps.** Update the milestone row in `plan/_index.md` (status, remaining human steps) and its "Open now" line. Add or update the rows of `plan/human-steps.md` for what only a person can do (real server, phone, hardware). Mark IMPROVEMENTS or review files for the findings the milestone closed (CLAUDE.md workflow 7). Run `python3 tools/fa.py check`.
10. **Status and log.** `python3 tools/fa.py status` (refreshes `STATUS.md`), then `python3 tools/fa.py log --topics M<n>,close --heading "M<n> closed" --text @log.txt` with what was run and the day's open items. Close the log's open items: each is done (with the commit) or listed as open. A log never ends mid-task.
11. **Self-learn.** Turn anything that went wrong and could recur into a rule in CLAUDE.md or a step in one of the milestone skills (`plan-milestone`, `run-wave`, this one), and note it in the log.
12. **Commit** when every command above passed: `git commit -m "<- close M<n>: summary"`, one line, no body, no trailer, staging explicit paths. Report anything not run.
