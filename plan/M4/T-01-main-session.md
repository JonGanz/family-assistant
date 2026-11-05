---
id: T-01
title: "Main session: plan, rules, workspace, regeneration, review checkpoints, integration"
milestone: M4
wave: 0
depends_on: []
agent: {model: opus, effort: medium}
status: done
owns: ["plan/**", "Cargo.toml", "apps/web/src/gen/**", "apps/web/src/views/views.test.ts"]
decisions: [D-093, D-094, D-095, D-096, D-097]
---

# T-01: Main session

Done when planning ends: `plan/M4.md`, `plan/M4/_rules.md`, the briefs, decisions D-093 to D-097 (questions Q-083 to Q-086), research R-004 and R-005, and `arch/AI.md` with the Frontend, Overall, Ingest and Platform updates.

Integration rows (each one is ticked in `plan/M4.md` *Task status* when done):
1. Before wave 1:
   - Add the members `crates/fa-llm`, `crates/fa-recipe` and `crates/fa-eval` to the root `Cargo.toml`, with an empty crate skeleton each so the workspace builds, plus the workspace dep `schemars = "1.2.3"`.
   - Run `cargo clean`, because `target/debug/deps` is over 15 GB.
   - Run `SQLX_OFFLINE=true cargo check --workspace`.
2. After wave 1:
   - Regenerate `apps/web/src/gen`.
   - Run `scripts/ui-shots.sh --gallery`.
   - Publish review artifact R1.
   - Delete `target/debug/incremental` and run `docker builder prune -f`.
3. After R1, append the feedback to T-05's brief under `## R1 feedback`.
4. After wave 2:
   - Split `views.test.ts` per area.
   - Run `ui-shots --all`.
   - Publish artifact R2.
   - Append the R2 feedback to the T-10 to T-14 briefs.
5. After wave 3:
   - Regenerate `gen`.
   - Fix the e2e selectors the tasks reported.
   - Run `ui-shots --all`.
   - Publish artifact R3.
   - Turn larger feedback into a fix-up task in wave 4.
6. Wave 5:
   - Remove dead components (`PageHeader`, `StateBlock`, `ErrorBanner`, `Tabs`) if they are unused.
   - Publish artifact R4.
   - Run the `close-milestone` skill.
