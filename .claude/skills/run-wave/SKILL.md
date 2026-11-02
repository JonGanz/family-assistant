---
name: run-wave
description: Run one wave of a milestone plan by launching its task briefs as parallel sub-agents, then integrate the results (build, test, regenerate types, statuses, log, commit). Use when the user says to run, launch or start a wave or a task of a milestone.
---

# Run a wave

Read `plan/M<n>.md` (the waves table and *Contracts fixed up front*), `plan/M<n>/_rules.md` and `plan/_rules-common.md`. Run this in the main session; the sub-agents never commit.

1. **Check the wave can start.** Every task in the wave has `status: todo`, and each task in its `depends_on` is `done` in its brief and in the plan's Task status table. A row marked "(main)" in the waves table (for example regenerating `apps/web/src/gen`) that the wave needs must be finished first. Run the cross-check of `plan-milestone` step 4 again for this wave's briefs: owned columns and tables, no overlapping `owns`, wire formats present.
2. **Disk (the one rule).** `df -h /`. Between waves delete `target/debug/incremental`, run `docker builder prune -f` (never prune volumes), and run `cargo clean` when `target/debug/deps` is over 15 GB. Agents stop below 6 GB free. Also `docker volume ls -f dangling=true` and note any volumes that exist before the wave.
3. **Launch** every task of the wave in **one message** with one Agent call each, so they run in parallel: the prompt is the brief file (tell the agent to read `plan/M<n>/T-NN-slug.md` and follow it, working directory the repo root); model Sonnet 5.5 and the effort from the brief's front-matter `agent:` (medium unless it says otherwise). Add to each prompt: edit only the `owns` paths, run no state-changing `git` commands, finish with the report format of `plan/_rules-common.md`. Set the brief's `status: running`. Any glob or bulk edit names its own files explicitly (a `T-0*.md` loop once rewrote another agent's briefs).
4. **Wait for the reports.** Do not poll and do not predict results. If a report asks for another task's paths or contradicts a contract, decide it in the main session and update the plan's contracts.
5. **Integrate** (main session, from the repo root). Run the checks the milestone's `_rules.md` names, against a fresh Postgres, never only the dev DB: `scripts/ci-local.sh --no-web` for Rust, docs and hygiene (it starts its own throwaway container), and from `apps/web` `corepack pnpm@9 run lint`, `typecheck`, `test` and `build` when web paths changed (not a bare `vue-tsc --noEmit`). If DTOs changed (T-02/T-03 style tasks), regenerate the web types, OpenAPI and client with the README command, then re-run the web checks:
   ```bash
   cargo test -q -p fa-domain && cargo run -q -p fa-server -- gen-openapi --out apps/web/src/gen/openapi.json && cd apps/web && corepack pnpm@9 run gen
   ```
   After editing the `Dockerfile`, check that every `COPY` source exists. Fix small issues yourself; larger ones become a follow-up brief in the same milestone or a finding for the next. Never quote an aggregate number (test counts and the like) without computing it from the output.
6. **Update statuses.** Per brief: `status: done` (or `blocked` with the reason), and add `## Outcome (T-NN report)` with the files built, the deviations and the follow-ups. Update the plan's *Task status* table to match. Brief and table must agree.
7. **Clean up** the disk (step 2) and make sure the throwaway containers of the wave are gone (`docker rm -f -v <name>`; `docker volume ls -f dangling=true` shows nothing new).
8. **Log.** `python3 tools/fa.py log --topics M<n>,wave-<k> --heading "M<n> wave <k>" --text @log.txt`: what ran, results (counts only from real output), what is left. Run `python3 tools/fa.py check` and `python3 tools/fa.py status`.
9. **Commit** only when the CI-equivalent checks passed: one line `git commit -m "<- M<n> wave <k>: summary"`, no body, no trailer. Stage the paths of the wave explicitly (the user's unrelated changes in the tree stay out). Report what was not run.
10. **Next wave.** Say which wave is next, or that only `close-milestone` remains.
