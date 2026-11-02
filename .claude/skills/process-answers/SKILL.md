---
name: process-answers
description: Process answered design questions in design/ into decisions, arch updates (md + drawio), resolved moves, index updates, and a daily log entry. Use when the user says to process answers/questions.
---

# Process Answers

Use the CLI `python3 tools/fa.py <subcommand> --flag value` (run `python3 tools/fa.py --help`, or `<subcommand> --help`). Flags are keywords only: positional arguments are rejected, so nothing can be swapped silently. Pass long text with `@path` (write the text to a scratch file first), or `--flag=-text` for text that starts with `-`. Never write `python3 -c` one-liners against `fa.py`, and don't hand-edit the indexes: the writers update them and stamp their `updated:` date. Text edits inside an existing file use the Edit tool (exact match).

1. Find the answered questions with `python3 tools/fa.py answered` (prints each question file and its answer text).
2. For each answered question:
   - Write one or more decisions with `python3 tools/fa.py new-d --slug S --title T --questions Q-NNN --affects arch/X.md,arch/Y.md --context @ctx.txt --decision @dec.txt --consequences @con.txt --alternatives @alt.txt` (`--id` defaults to the next free number; `--questions` is empty for a direct user instruction; `--supersedes D-0NN` records the link). It creates `decisions/D-NNN-slug.md` with the front-matter from CLAUDE.md and the sections Context, Decision, Consequences and Alternatives considered, and appends the row to `decisions/_index.md`.
   - Resolve the question with `python3 tools/fa.py resolve --q Q-NNN --decisions D-NNN[,D-MMM] [--date YYYY-MM-DD]`. It sets `status: resolved`, `resolved:` and `decisions:`, runs `git mv` into `design/resolved/`, and rebuilds `design/_index.md`.
3. Re-derive the affected `arch/*.md` from the requirements and all accepted decisions. Do not just patch the old design. Then update `arch/src/<Name>.py` and regenerate the `.drawio` in the same pass (`python3 arch/src/<Name>.py`). If a decision overturns an earlier one, run `python3 tools/fa.py supersede --old D-0NN --new D-0MM --note TEXT` (marks the old one superseded, adds the banner, updates the index row and sets `supersedes:` on the new one). To record a refinement without superseding, use `annotate --d D-0NN --note TEXT`.
4. Some answers will open new questions. Create the next batch with `python3 tools/fa.py new-q --slug S --title T --area A --batch B --question @q.txt --option "..." --option "..."` (`--n` defaults to the next free number, found across `design/` and `design/resolved/`; `next-id --kind q|d` prints it). It also rebuilds `design/_index.md`. When writing options, include a "configurable with default X" option whenever the behaviour could vary.
   Don't read a user's "we don't need X" as "we must not have X". If the wording is ambiguous, choose the permissive and configurable reading, or ask.
5. Update `arch/_index.md` by hand. `design/_index.md` and `decisions/_index.md` are already done by the commands above (`python3 tools/fa.py rebuild-design-index` regenerates the design one from the question files).
6. Append to today's log with `python3 tools/fa.py log --topics a,b --heading H --text @log.txt`. It creates `log/<today>.md` (with front-matter) and the `log/_index.md` line when the day is new, and merges the topics when it exists. Close the log's open items before handing back.
7. If one answer settles another open question, or a question was only partly answered, record both in a single decision and ask about the missing part in the next batch.
8. Answers sometimes contain the user's own ideas or questions ("would it make sense to..."). Evaluate the idea honestly in a decision. If you adopt it, say why. If it changes an earlier decision, supersede that decision. Answer the question directly in the summary to the user.
9. When several arch docs are affected, re-derive them in parallel with one fork per doc (strict ownership: the md, its `arch/src/*.py` and `.drawio`; no indexes). Afterwards, grep all of `arch/` for terms that the decisions removed, then bring the open plan and `IMPROVEMENTS.md` in line with the decisions.
10. Run `python3 tools/fa.py check`, then `python3 tools/fa.py status` to refresh `STATUS.md` (open questions, open human steps, unfinished tasks, newest log).
11. Self-learn: if this run uncovered a repeated pattern, update CLAUDE.md or this skill.
