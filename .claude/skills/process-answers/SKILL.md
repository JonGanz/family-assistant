---
name: process-answers
description: Process answered design questions in design/ into decisions, arch updates (md + drawio), resolved moves, index updates, and a daily log entry. Use when the user says to process answers/questions.
---

# Process Answers

1. Use `grep -L` or a read to find the `design/Q-*.md` files where `## Answer` has non-placeholder text. Skip any that are still blank.
2. For each answered question:
   - Write one or more `decisions/D-NNN-slug.md` files using the next free number. Include the front-matter from CLAUDE.md, and add the sections Context, Decision, Consequences, and Alternatives considered.
   - Fill in `decisions: [D-NNN]` on the question and set `status: resolved` and `resolved: <date>`.
   - `git mv` the question into `design/resolved/`.
3. Re-derive the affected `arch/*.md` from the requirements and all accepted decisions. Do not just patch the old design. Then update the matching `.drawio` in the same pass. If a decision overturns an earlier one, set the old one to `superseded` and cross-link the two.
4. Some answers will open new questions. Create the next batch of `Q-NNN` files for them.
5. Update `design/_index.md`, `decisions/_index.md`, and `arch/_index.md`.
6. Append to `log/<today>.md`, and add a line for it to `log/_index.md` if the file is new.
7. Self-learn: if this run uncovered a repeated pattern, update CLAUDE.md or this skill.
