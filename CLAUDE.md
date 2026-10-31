---
title: Claude Working Rules — Family Assistant
updated: 2026-10-23
---

# Family Assistant — Rules of Work

`PROJECT.md` holds the requirements, and the user owns it. Read it at the start of any session that touches scope. Do not edit it unless the user asks.

## Where things live
| Dir | Purpose | Index |
|---|---|---|
| `arch/` | Architecture designs. Each `X.md` has a matching `X.drawio` (UML 2) | `arch/_index.md` |
| `design/` | Open questions, one file each (`Q-NNN-slug.md`) | `design/_index.md` |
| `design/resolved/` | Questions that are answered and processed | same index |
| `decisions/` | Decision records (`D-NNN-slug.md`) taken from the answers | `decisions/_index.md` |
| `log/` | Daily logs (`YYYY-MM-DD.md`) | `log/_index.md` |
| `research/` | External research notes (`R-NNN-slug.md`) that decisions cite | `research/_index.md` |
| `plan/` | Milestone plans (`M<n>.md`) and sub-agent task briefs (`M<n>/T-NN-slug.md`) | `plan/_index.md` |

**Start each session by reading the `_index.md` files, not the whole tree.** Open individual files only when you need them.

## Front-matter (required on every managed .md)
- Questions: `id, title, status (open|answered|resolved), batch, area, created, resolved, decisions: []`
- Decisions: `id, title, status (accepted|superseded), date, source_questions: [], supersedes, superseded_by, affects: [arch files]`
- Arch: `title, scope, status (draft|accepted), updated, drawio, depends_on: []`
- Arch docs may also carry `milestone:` and `drawio_src:`.
- Logs: `date, topics: []`
- Research: `id, title, date, method, verified, informs: []`
- Plans: `title, milestone, status, updated, arch, decisions`. Tasks: `id, title, milestone, wave, depends_on, agent, status (todo|running|done|blocked), owns`

## Workflows
1. **Asking questions:** create a `design/Q-NNN-*.md` from the template in `design/_index.md`. Group them into batches (`batch: N`). Number them in sequence and never reuse a number. Add each one to the index.
2. **Processing answers** (when told to): run the `process-answers` skill (`.claude/skills/process-answers/SKILL.md`). In short: read any question that has text under `## Answer`, write `D-NNN` decision(s), update the arch md and drawio together, `git mv` the question to `design/resolved/` with `status: resolved`, update all the indexes, and write a log entry.
3. **Re-evaluate freshly.** When answers come in, re-derive the affected architecture from the requirements and *all* decisions. Do not just patch the previous design, because earlier paths must not anchor new choices. Mark replaced decisions `superseded`. Do not delete them.
4. **Arch md ⇄ drawio parity.** If you change one, change the other in the same step. Draw diagrams in UML 2 notation (component, deployment, sequence, class). Diagrams are generated: edit `arch/src/<Name>.py`, which uses the helpers in `tools/drawio.py`, then run `python3 arch/src/<Name>.py`. Never hand-edit the .drawio file. The helpers include `Page.box/edge` (components, nodes) and `cls`/`assoc` (UML class: kinds assoc, composition, aggregation, generalization, dependency).
5. **Log daily.** Append to `log/YYYY-MM-DD.md`. Record what you did, what was discussed, and what is still open. **Before handing back, close the log's open items:** anything written as "in progress" or "still to do" is either done (say so, with the commit) or listed under the day's open items. A log must never end mid-task.
6. **Lint the design tree.** `python3 tools/fa.py check` verifies front-matter, status enums, id/filename parity, duplicate numbers and index ⇄ file parity. CI and `scripts/ci-local.sh` run it. Run it after any edit to a managed `.md`.
7. **Reviews.** A review session (process, code, direction, UI) is time-boxed. Its findings go into `plan/reviews/YYYY-MM-DD-<slug>.md` as tables with an ID, size (QUICK or LARGER) and status, plus a "suggested order of work". Findings that need the user's decision become questions; QUICK items may be fixed in the session by sub-agents with strict ownership; LARGER items become briefs in the next milestone. Add the review to `plan/_index.md` and log it.
8. **Closing a milestone** (checklist): `scripts/ci-local.sh`, `scripts/e2e.sh` and `scripts/compose-smoke.sh` green; 0 dangling Docker volumes; every task `done`; `## Outcome` with known gaps in `plan/M<n>.md`; README roadmap and steps verified by running them; `plan/_index.md` updated with the human steps that remain; log entry.

## Standing technical constraints
- Everything runs locally. Any AI work uses local LLMs on 32 GB RAM and an 8 GB RTX 3070. Budget for roughly 7–8B models at Q4 and small embedding models.
- Go deterministic first: rules, headers, parsers, and schema.org data. Use the LLM only for what rules cannot handle, and make it return structured JSON.
- **When writing question options, include a "configurable, default X" option whenever the behaviour could reasonably vary.** The user has said that configurability is their default answer.
- **This repo is public (AGPL, D-035).** Never write household specifics (names, schools, addresses, emails) into docs, logs, decisions, or fixtures.
- **Configurable, not hard-coded.** Design for *any* household and *any* hardware, and treat the reference setup (a Ubuntu core node plus one RTX 3070 gaming PC) as just the default profile. Optional or hardware-dependent features get a toggle with a suggested default, and behaviour that is likely to vary should be data that can be edited in the UI (see D-026, D-029). The user has asked for this repeatedly.
- Intelligence comes in tiers: rules, then System 1, then System 2. **"System 1" means System One decision models** (https://systemonemodels.org/, see R-001 and D-024), not a small generative LLM. System 2 is the generative LLM on the GPU node.
- **If the user names an unfamiliar term, product, or site, research it (a sub-agent writes up an `R-NNN`) before designing around it. Don't guess.**
- Design around family members. Each member has a role (adult or child) and has interests, hobbies, and responsibilities.

## Implementation via sub-agents
- Each milestone plan fixes the **contracts** (crates, migrations ownership, API table) up front, so tasks in the same wave can run in parallel.
- Launch each task as a sub-agent (Sonnet 5.5, medium effort unless the plan says otherwise), with the brief file as its prompt. Run the tasks of a wave in parallel, and don't start a wave before its dependencies are done.
- Ownership is strict: an agent edits only its `owns` paths. Between waves, the main session integrates (build, test), fixes small issues, updates task `status`, and logs the result.
- **Verify every external name you write into a design or brief**, including image names, crate and package names, and API paths. If you're unsure, check it (for example `docker pull`, a registry search, or a research sub-agent). A wrong image name, `binwiper/ntfy` instead of `binwiederhier/ntfy`, went from Platform.md into the compose file.
- **Before closing a milestone, run `scripts/e2e.sh` yourself.** In M1, every sub-agent passed its unit tests, but a Calendar view that never rendered events showed up only in e2e.
- **Run CI-equivalent checks against a fresh Postgres** (a throwaway container with `SQLX_OFFLINE=true`), never only the long-lived dev DB. Its state (existing roles, migrated tables) hides failures.
- **Once a migration has been deployed anywhere, never edit it.** Add a new migration instead. (Before release, an edit is OK if every DB's `_sqlx_migrations` checksum is updated.)
- **Never quote aggregate numbers (test counts and the like) without computing them from the output.**
- Sub-agent build rules (fmt, disk, containers, wire formats, ts-rs) live in [`plan/_rules-common.md`](plan/_rules-common.md); every brief links it. **Disk (one rule):** agents stop below 6 GB free; between waves the main session deletes `target/debug/incremental`, runs `docker builder prune -f` (never prune volumes), and runs `cargo clean` when `target/debug/deps` passes 15 GB.
- When agents (forks included) work in parallel in one directory, any glob or bulk edit must name its own files explicitly. A `T-0*.md` loop once rewrote another fork's briefs.
- When a milestone changes both `fa-domain` types and `fa-store`, give them to **one** task in the first wave: every other crate depends on both, and splitting them across waves breaks the build in between.
- Cross-check briefs before launching: every column, table, or dependency a task needs must be owned and created by an earlier or same-wave task.
- For every new column, the plan names what writes it (route, TOML key, UI) and its default must match the arch, and every time or date DTO field in the plan states its wire format. M2 shipped a column nothing could set and a panel that parsed `"HH:MM"` with `Date.parse`.

## Decisions vs design choices
Only answers to questions (and ideas the user puts inside answers) become `D-NNN` records, with one exception: a direct user instruction that changes scope or security (D-064 is the model) is recorded as a decision with `source_questions: []` and a note that it came from an instruction. Technical choices made inside an arch doc, such as library picks, live in that doc. If the user challenges one, it becomes a question.

## Git
The user commits. Their commit messages use `->` for the user's input and `<-` for Claude's output. Don't commit unless asked. Use `git mv` for moves so history is kept.

## README (user-facing, keep current)
`README.md` is the entry point for people using or contributing to the project. **Update it in the same change** whenever any of these change:
- setup steps or prerequisites;
- commands (CLI, scripts);
- environment variables or ports;
- the repository layout;
- the deployment story;
- milestone status (the Roadmap table).

Every milestone's definition of done includes "README updated and its steps verified by actually running them". Sub-agent briefs that add commands or setup must either update the README (if they own it) or report the README changes needed.

## Tooling
- `tools/drawio.py`: the diagram generator.
- `tools/fa.py`: design-workflow helpers for questions, decisions, indexes, and asserted edits. Use them instead of ad-hoc scripts.

## Self-learn
After each session, look for repeated work or patterns. Turn them into rules in this file or into skills under `.claude/skills/`. Note any such change in the day's log.
