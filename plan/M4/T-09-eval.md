---
id: T-09
title: "fa-eval (basic) and the eval subcommand"
milestone: M4
wave: 4
depends_on: [T-06, T-08]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-eval/**", "bins/fa-server/src/eval.rs", "bins/fa-server/src/cli.rs (the Eval variant only)", "bins/fa-server/src/main.rs (the Eval dispatch arm only)"]
decisions: [D-028, D-094, D-024]
---

# T-09: `fa-eval` (basic) and the `eval` subcommand

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. The labelling view (M3, D-028) builds an eval sample of about 200 real messages with human decisions. M4 needs a basic eval: replay that sample through the pipeline with rules only, then with rules plus the `llm` stage, and compare accuracy, coverage and latency per decision point.

**You never run the eval against the dev database.** It holds a real household. Running it on real data is a human step; your tests use fixtures.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md` (real-data rule).
- `plan/M4.md`: *Eval (T-09)*; `arch/AI.md` §10.
- Decisions D-028, D-094.
- Code:
  - `crates/fa-mail/src/labelling.rs` (the eval sample is `message.in_eval_sample = true`; `EVAL_TARGET`; `MessageRepo::eval_progress`);
  - the `decision` table (`crates/fa-store/migrations/20261027000001_m3_ingest.up.sql`); the truth is the latest `stage_kind = 'human'` decision for a subject and decision point;
  - `crates/fa-pipeline/src/lib.rs` (`Evaluator`, rules candidates, `record_stage`);
  - the `mail.llm` job module in `fa-mail` (T-06) for how the prompt and `MailLlmResult` are built: **reuse its builder** (`pub` it if needed by reporting; do not copy the prompt);
  - `bins/fa-server/src/cli.rs` (the clap `Command` enum) and `main.rs`.

## You own
`crates/fa-eval/**`, `bins/fa-server/src/eval.rs`, the new `Eval` variant in `bins/fa-server/src/cli.rs`, and its dispatch arm in `main.rs`. Nothing else in `bins/`.

## Work
1. **`fa-eval`:** `pub async fn run(conn, household, dp: DecisionPointKey, opts: EvalOpts { with_llm: bool, limit: Option<u32> }, llm: Option<&fa_llm::Provider>) -> Result<EvalReport>`.
   - For each sample message that has a human truth for `dp`, take the rules candidates in-process.
   - With `with_llm`, call the `Provider` (`purpose = "eval"`) for the messages whose rules fall below the threshold. Use the same prompt builder as `mail.llm`, limited to `dp`.
   - It **writes no decisions** and enqueues nothing.
   - `EvalReport { dp, items, per_stage: Vec<StageReport { stage: StageKind, decided, correct, p50_ms, p95_ms }>, accuracy: f64, coverage: f64 }`: `coverage` is the share decided before `human`, and `accuracy` is correct over decided.
   - `member_link` is multi-valued: count an item correct when the set of members at or above the threshold equals the human set.
2. **`fa-server eval --household <uuid> --decision-point <dp> [--with-llm] [--limit N] [--json]`:** prints a plain table, or `--json` prints the `EvalReport` as JSON. It uses `tx_for(household)`. Without an enabled endpoint, `--with-llm` exits with a clear error and code 2.
3. **Tests** (`crates/fa-eval/tests/`, fresh container on port 55409): fictional messages with `in_eval_sample = true` and human decisions; rules that get some right; an axum stub LLM that answers the rest.
   - Rules-only and with-LLM numbers are correct.
   - No `decision` rows are added.
   - `llm_call` rows have `purpose = "eval"`.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-eval -p fa-server`, `cargo clippy -p fa-eval -p fa-server --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes.
- [ ] `cargo run -p fa-server -- eval --help` shows the flags. Do **not** run it against the dev DB.
- [ ] The report lists:
  - the command synopsis, for T-17 and the human step;
  - the prompt-builder function you reused and whether it had to become `pub`;
  - "list what outside your paths needs changing";
  - README changes needed (the `eval` command line, for T-17).
