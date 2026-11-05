---
id: T-08
title: "fa-api AI routes and fa-server wiring"
milestone: M4
wave: 3
depends_on: [T-03, T-06, T-07]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-api/**", "bins/fa-server/**"]
decisions: [D-093, D-094, D-029, D-006]
---

# T-08: `fa-api` AI routes and `fa-server` wiring

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M4's track B has its tables (T-03), the `fa-llm` client and `Provider` (T-04, T-06), the `llm` stage and `mail.llm` job (T-06), and `fa-recipe` (T-07). You expose them over the API for the Settings → AI screen (built in wave 4) and wire the `Provider` into the server, so that the workspace builds again.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`.
- `plan/M4.md`: *API (T-08)*, *`fa-llm`* (`Provider`) and *`fa-recipe`*.
- `arch/AI.md` §8 (API), §7 (failure modes) and §11 (security); `arch/Platform.md` §4 (API conventions and naming, M3.7 C-13) and §5.
- Decisions D-093, D-094; D-006 (adults only).
- The reports of T-03, T-06 and T-07 (`Provider::new` arguments, the `MailDeps.llm` field, the `RecipeError` variants).
- Code:
  - `crates/fa-api/src/lib.rs` (router, `AppState`, OpenAPI);
  - `crates/fa-api/src/mail.rs` (handler style, `require(&auth.actor, Action::…, &Resource::Household)`, `ApiResult`, `ApiQuery`);
  - `crates/fa-api/src/system.rs` or whichever module serves the settings routes;
  - `crates/fa-auth` (Actions; add one only if no fitting adult-settings action exists, and report it);
  - `bins/fa-server/src/wiring.rs` and `serve.rs`.

## You own
`crates/fa-api/**` and `bins/**`, except `bins/fa-server/src/eval.rs` (T-09, wave 4). If `fa-auth` needs a new `Action`, report it rather than editing, unless an existing adult-settings action fits. Prefer the existing one.

## Work
1. **`crates/fa-api/src/ai.rs`**, with every route in the plan's *API* table: the exact paths, bodies and responses, adult-only, `utoipa` documented, and registered in `lib.rs` under the existing `/api/v1` prefix.
   - **`PUT /ai/llm-endpoint`:**
     - validate `base_url` (`http` or `https`, no query, no userinfo; trim a trailing slash), a non-empty `model`, and `timeout_ms` in 1000..=600000;
     - upsert the row; handle the key: absent keeps it, `null` deletes the secret, a string seals it with the C-10 AAD;
     - call `Provider::invalidate`;
     - audit the change (the existing audit pattern for settings) **without** the key.
   - **`POST /ai/llm-endpoint/test`:** build a transient `LlmClient` from the body, filling missing fields from the stored row (and the stored key), and call `ping()`. Record an `llm_call` with `purpose = "endpoint.test"`. Return `{ok, latency_ms, model_found, error}`, where `error` is `LlmError::kind()`. It never returns 5xx for an unreachable server: that is `ok: false`.
   - **`GET /ai/llm-endpoint/models`:** 502 `llm_unavailable` on `Unavailable` or `Timeout`.
   - **`GET`/`PUT /ai/pipeline`:** the four email decision points. `PUT` calls `Pipeline::set_llm_stage`, validates the threshold (0..=1) and returns 404 for another point.
   - **`GET /ai/stats?days=`:** days default 7, clamped to 1..=90. It uses `LlmCallRepo::stats_since` and `DecisionRepo::stage_counts_since`. `since` is RFC 3339.
   - **`POST /ai/recipes/extract`:** exactly one of `url` or `html`, else 422 `bad_request`. The fetch uses an `Http` with private hosts blocked (`Http::with_policy(true, false)`). Responses: 422 `no_recipe`, 503 `llm_unavailable`, and a fetch error is 422 `fetch_failed` (no URL echoed in logs). It records `purpose = "recipe.extract"` through the `Provider` when the LLM is used.
2. **DTOs:** request and response structs live in `fa-api` (or `fa-domain` if that is where the API DTOs live today; follow the existing pattern), with ts-rs export so that `gen/` picks them up. All time fields are RFC 3339.
3. **`fa-server` wiring:**
   - build one `fa_llm::Provider` (with `fa_llm::default_http()`) in `wiring.rs`;
   - pass it to `AppState` and to `fa_mail::MailDeps { llm, .. }`;
   - make sure `mail.llm` and `llm.prune` (from `fa_mail::handlers()` and `default_schedules()`) are registered by the existing loops.
   The workspace must build again.
4. **Logging:** never log mail content, LLM prompts or responses, or the API key. Recipe URLs are logged as the host only.
5. **API tests** (`crates/fa-api/tests/`), with an axum stub LLM and a fresh DB:
   - a child or device gets 403 on every `/ai/*` route;
   - PUT, then GET of the endpoint never returns the key (`has_api_key` flips; `api_key: null` clears it);
   - `test` against the stub gives `ok` and `model_found`, and against a closed port gives `ok: false, error: "unavailable"`;
   - the `pipeline` PUT creates a new version;
   - `stats` shape;
   - recipe `html` with JSON-LD gives `origin: "jsonld"`; with no JSON-LD and the endpoint disabled it gives 503;
   - `url` pointing at `127.0.0.1` gives 422 `fetch_failed`.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] On a fresh container (port 55408): `cargo test -p fa-api -p fa-server` passes.
- [ ] `cargo clippy -p fa-api -p fa-server --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` is **green**.
- [ ] The OpenAPI document includes every `/ai/*` route. The main session regenerates `apps/web/src/gen` afterwards; do not edit `gen/`.
- [ ] The report lists:
  - routes and DTO names, for T-15 and T-16;
  - any `fa-auth` action needed;
  - the error codes;
  - "list what outside your paths needs changing";
  - README changes needed: none beyond T-17's AI section.
