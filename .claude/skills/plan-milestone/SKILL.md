---
name: plan-milestone
description: Turn a scope (decisions, review findings or a user request) into a milestone plan plus rules and one sub-agent brief per task. Use when the user asks to plan a milestone or to turn review findings into tasks.
---

# Plan a milestone

Models to copy: `plan/M3.6.md` with `plan/M3.6/_rules.md` and its briefs; `plan/M3.7.md` with `plan/M3.7/`. Shared ground rules live in `plan/_rules-common.md`: never copy them into a milestone, only add deltas. Start by reading `plan/_index.md`, `CLAUDE.md` ("Implementation via sub-agents"), `plan/_rules-common.md` and the arch sections and decisions the scope touches. If the scope needs a decision that does not exist, ask a question first (`python3 tools/fa.py new-q ...`, see the `process-answers` skill) and plan only what is settled. Never write household specifics (D-035).

1. **Pick the number and scope.** The next `M<n>` follows `plan/_index.md` (a follow-up milestone between two others is `M3.7`). List the source of the scope (decision ids, review finding ids, the user's words). Anything not settled goes under "Out of scope".
2. **Write `plan/M<n>.md`.** Front-matter: `title, milestone, status, updated, arch, decisions` (plus `executor:`, as M3.6 and M3.7 do). Sections, in this order:
   - **Why**: the scope's source and what was checked against the code.
   - **Done means**: a numbered list a person can verify. The last item is always the CLAUDE.md workflow 8 checklist (`scripts/ci-local.sh`, `scripts/e2e.sh`, `scripts/compose-smoke.sh` green, README updated and its steps verified).
   - **Contracts fixed up front**: a **waves table** (wave, tasks, needs; the main session's regeneration steps and the integration wave appear as rows), then every name, type, route, migration file name and wire format that two tasks share. Each date or time DTO field states its format (`"HH:MM"` local, RFC 3339, `YYYY-MM-DD`). Each new column names what writes it (route, TOML key or UI) and its default, which must match the arch doc.
   - **Out of scope**.
   - **Task status**: a table with every task and `todo`.
3. **Cut the tasks.** One owner per path; `owns` lists globs and no two tasks in the same wave overlap. Work that changes both `fa-domain` types and `fa-store` is **one** task in the first wave (every other crate depends on both). Web tasks that need regenerated types go after the main session's regeneration row. Read-only tasks (reviews) own only their output file.
4. **Cross-check the list before any launch** (write the result into the plan's `## Contracts` or as a note in the log):
   - every column, table, crate or package a task needs is created by an earlier or same-wave task (or already exists: check the code, not memory);
   - every external name in a brief (image, crate, package, API path) was verified (`cargo add --dry-run`, `docker manifest inspect`, official docs) and is not a guess;
   - every new column has a writer and a default that matches the arch;
   - every time or date field has a wire format;
   - no two tasks in a wave own the same path; `CLAUDE.md`, the README and the root `Cargo.toml` have exactly one owner (usually the main session or a docs task);
   - tasks that add commands, env vars, ports or setup either own the README or are told to report the README change.
5. **Write `plan/M<n>/_rules.md`.** Front-matter `title, milestone`. It starts "Read `plan/_rules-common.md` first. These are the M<n> deltas." Then only what differs: which checks can run locally, the contracts pointer, the colour or data rules, the rustc note. No restating of the common rules.
6. **Write one brief per task**, `plan/M<n>/T-NN-slug.md`. Front-matter: `id, title, milestone, wave, depends_on, agent ({model: sonnet, effort: medium|high}), status: todo, owns, decisions`. Body: a one-paragraph role statement (what project, which milestone), `## Read first` (the common rules, the milestone rules, the plan section, decisions, arch sections and the files to read), `## You own`, `## Work` (numbered, concrete, with names and formats from the contracts), `## Acceptance` (checkboxes: the exact commands, the tests to add, and the report line "list what outside your paths needs changing"). T-01 is the main session's own task (plan, rules, regeneration) and is already `done` when the briefs are written.
7. **Update `plan/_index.md`**: add the milestone row (plan, status, task list). If a person has to do something at the end, add its rows to `plan/human-steps.md`.
8. **Check and log.** Run `python3 tools/fa.py check`. Then `python3 tools/fa.py log --topics plan,M<n> --heading "M<n> planned" --text @log.txt`, and list in the text any open items. Hand back with the wave order and what you could not settle.
9. **Self-learn.** If a new gap in a brief showed up in the cross-check, add it to CLAUDE.md or this skill.
