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

**Start each session by reading the `_index.md` files, not the whole tree.** Open individual files only when you need them.

## Front-matter (required on every managed .md)
- Questions: `id, title, status (open|answered|resolved), batch, area, created, resolved, decisions: []`
- Decisions: `id, title, status (accepted|superseded), date, source_questions: [], supersedes, superseded_by, affects: [arch files]`
- Arch: `title, scope, status (draft|accepted), updated, drawio, depends_on: []`
- Logs: `date, topics: []`

## Workflows
1. **Asking questions:** create a `design/Q-NNN-*.md` from the template in `design/_index.md`. Group them into batches (`batch: N`). Number them in sequence and never reuse a number. Add each one to the index.
2. **Processing answers** (when told to): run the `process-answers` skill (`.claude/skills/process-answers/SKILL.md`). In short: read any question that has text under `## Answer`, write `D-NNN` decision(s), update the arch md and drawio together, `git mv` the question to `design/resolved/` with `status: resolved`, update all the indexes, and write a log entry.
3. **Re-evaluate freshly.** When answers come in, re-derive the affected architecture from the requirements and *all* decisions. Do not just patch the previous design, because earlier paths must not anchor new choices. Mark replaced decisions `superseded`. Do not delete them.
4. **Arch md ⇄ drawio parity.** If you change one, change the other in the same step. Draw diagrams in UML 2 notation (component, deployment, sequence, class). Diagrams are generated: edit `arch/src/<Name>.py`, which uses the helpers in `tools/drawio.py`, then run `python3 arch/src/<Name>.py`. Never hand-edit the .drawio file.
5. **Log daily.** Append to `log/YYYY-MM-DD.md`. Record what you did, what was discussed, and what is still open.

## Standing technical constraints
- Everything runs locally. Any AI work uses local LLMs on 32 GB RAM and an 8 GB RTX 3070. Budget for roughly 7–8B models at Q4 and small embedding models.
- Go deterministic first: rules, headers, parsers, and schema.org data. Use the LLM only for what rules cannot handle, and make it return structured JSON.
- Optional or hardware-dependent features get a **toggle**, with a suggested default (the user keeps asking for this).
- Intelligence comes in tiers: rules, then System 1 (core node, always on), then System 2 (GPU node, batched). See D-022.
- Design around family members. Each member has a role (adult or child) and has interests, hobbies, and responsibilities.

## Git
The user commits. Their commit messages use `->` for the user's input and `<-` for Claude's output. Don't commit unless asked. Use `git mv` for moves so history is kept.

## Self-learn
After each session, look for repeated work or patterns. Turn them into rules in this file or into skills under `.claude/skills/`. Note any such change in the day's log.
