---
id: T-01
title: "Main session: arch, process items, briefs, baseline run"
milestone: M3.5
wave: 0
depends_on: []
agent: main session
status: done
owns: ["arch/**", "plan/**", "tools/**", "CLAUDE.md", ".claude/**"]
decisions: [D-065, D-066]
---

# T-01: Main session: arch, process items, briefs, baseline run

You are a sub-agent implementing one task of milestone M3.5 (onboarding and shakedown) of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend.

## Read first
- `plan/_rules-common.md` and `plan/M3.5/_rules.md` (mandatory ground rules)
- `IMPROVEMENTS.md` (process section), `plan/reviews/2026-10-26-repo-review.md`

## You own
- `arch/**`
- `plan/**`
- `tools/**`
- `CLAUDE.md`
- `.claude/**`

## Work
Done on 2026-10-26: arch re-derived (Platform v4, Frontend v4, Overall v8, Feeds/Ingest v1.1, Domain v5.1), briefs written, P-09 (`plan/_rules-common.md` + `plan/M3.5/_rules.md`) and P-10 (`decisions:` required in task front-matter from M3.5 on, enforced by `fa.py check`).

Still to do before W1:
1. **P-14:** move implementation gotchas (ts-rs, wire formats, disk thresholds) from `CLAUDE.md` into `plan/_rules-common.md`, keeping one disk rule; CLAUDE.md links to it.
2. **P-08b:** backfill `## Outcome` in `plan/M0.md` and `plan/M1.md` from the logs.
3. **P-02:** `plan/human-steps.md`: one row per H-01..H-14 with status and where it is verified.
4. **P-15:** a negative fixture test for `fa.py check` (a bad Q file in a temp dir), and validate `.github/workflows/ci.yml` as YAML.
5. **P-16:** run `scripts/ci-local.sh`, `scripts/e2e.sh`, `scripts/compose-smoke.sh` on the current tree; record the results in the log as the M3.5 baseline. Check `df -h /` first.
6. Between waves: integrate (build, test), run `cargo sqlx prepare` if needed, regenerate `apps/web/src/gen` after T-06 (`fa-server gen-openapi` + the web type generator), set task `status`, write the wave outcomes into the next briefs, log.

## Acceptance
- [ ] `python3 tools/fa.py check` clean
- [ ] baseline suite results logged
- [ ] P-02, P-08b, P-14, P-15 done
