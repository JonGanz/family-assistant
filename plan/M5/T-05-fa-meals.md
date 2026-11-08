---
id: T-05
title: "fa-meals: normaliser, recipe, plan, template and grocery services, jobs, seeding"
milestone: M5
wave: 2
depends_on: [T-02, T-04]
agent: {model: sonnet, effort: high}
status: todo
owns: ["crates/fa-meals/Cargo.toml", "crates/fa-meals/src/**", "crates/fa-meals/tests/**"]
decisions: [D-015, D-021, D-109, D-111, D-112, D-113, D-114, D-115, D-116, D-118, D-119]
---

# T-05: `fa-meals`

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M5 adds recipes, a meal plan with templates, and one living grocery list. You write the crate that holds all of its behaviour: the deterministic ingredient normaliser, the services the API calls, the background jobs, and the per-household defaults. The rule from CLAUDE.md applies throughout: deterministic first, and the LLM only proposes.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`, *Contracts fixed up front*: *Seed data format*, *Database*, *Domain types*, *`fa-meals`*, and the *API* table (each row is one service method).
- `arch/Meals.md` §4 (all of it: import, normaliser, plan, templates, generation, plan changes, the living list) and §8 (jobs).
- `research/R-007-ingredient-normalisation.md` §1, §3 and §5.
- T-02's report (the repo fns) and the code in `crates/fa-store/src/repo/` for the meals repos.
- Patterns to copy:
  - `crates/fa-tasks/src/service.rs` (a service holding a `Store`, `tx_for`, `ChangeBus::publish`);
  - `crates/fa-tasks/src/jobs.rs` and `crates/fa-mail/src/process/catchup.rs` (`JobHandler`, `handlers()`, `default_schedules()`);
  - `crates/fa-mail/src/process/llm.rs` (`Provider::call::<T>(household, purpose, &ExtractRequest)`);
  - `crates/fa-weather/src/defaults.rs` (`ensure_default_rules`: an idempotent seed);
  - `crates/fa-api/src/ai.rs` `extract_recipe` (fetch, JSON-LD, then the LLM with `max_input_chars`, `record_call`);
  - `crates/fa-api/src/feeds.rs` `sniff_image`.

## You own
`crates/fa-meals/**` except `seed/**` and `tests/seed_data.rs` (T-04). You take over `Cargo.toml` and `src/lib.rs` from the main session. Keep `pub mod seed_data;` and its file as they are.

## Work
1. **`Cargo.toml`** (`workspace = true`): `fa-domain`, `fa-store`, `fa-recipe`, `fa-llm`, `fa-jobs`, `fa-connectors`, `sqlx`, `serde`, `serde_json`, `schemars`, `thiserror`, `tracing`, `chrono`, `uuid`, `url`, `sha2` (or what `feeds.rs` uses), `pluralizer`, `image`.
   Dev-dependencies: `tokio`, `axum` (the LLM stub), `sqlx` with test features.
2. **`normalise` module** (pure, no I/O, table-tested):
   - `parse_line(raw)`: wraps `fa_recipe::parse_ingredient`.
   - `clean_item(item, descriptors, singular_rules) -> (item, note)`: lower case, ASCII folding, descriptor removal, parentheticals and trailing preparation into the note, singular via `pluralizer` plus the exceptions.
   - Unit table: maps the parser's units to `(BaseUnit, factor)`. Volume in ml (tsp 4.92892, tbsp 14.7868, cup 240, fl oz 29.5735, pint 473.176, quart 946.353, gallon 3785.41, l 1000, ml 1). Weight in g (oz 28.3495, lb 453.592, kg 1000, g 1). Count (clove, can, piece, slice, stick, bunch, head, sprig, pinch and none) stays in its own unit name.
   - `to_base(qty, unit) -> Option<(f64, BaseUnit or count unit)>`.
   - `display(qty, unit, Units) -> String`, rounding per `arch/Meals.md` §4.2 step 4. Metric shows ml/l and g/kg. Imperial shows tsp/tbsp/cup and oz/lb, with fraction glyphs ¼ ½ ¾ ⅓ ⅔.
3. **Matching** (`IngredientMatcher`): exact, then alias, then `IngredientRepo::nearest` at or above `meals_settings.trigram_threshold`, else unmatched. Saving a recipe sets each line's `qty`, `unit`, `item`, `note`, `ingredient_id` and `match_state`. Unmatched lines enqueue `ingredient.llm` when `ingredient_llm` is on and an LLM client exists.
4. **`RecipeService`:**
   - list (search: title trigram or ILIKE, plus ingredient name; tag; favourite; cursor), get, create, update (If-Match version), delete (entries keep the title as free text, same transaction), favourite, rating, line match (adds the alias), image get, upload and delete;
   - `import`: `{url}` uses `Http::with_policy(true, allow_private_hosts)` with `fa_recipe::extract`. `{text}` uses `fa_recipe::from_llm` on text truncated to `AiSettingsRepo` `max_input_chars`. Record LLM calls the way `ai.rs` does, with purpose `"recipe.extract"`.
5. **`PlanService`:**
   - range (at most 62 days), add, update, delete, swap/move;
   - writes are only allowed from today (household tz) to the horizon end; earlier dates are `Invalid{field: "date"}`;
   - every write computes `grocery_impact` (§4.6) for touched entries with unchecked, unarchived contributions;
   - templates: list, get, create, update, delete, `preview`, and `apply` (one transaction, `keep` honoured, a `meals_change` row with the inverse).
6. **`GroceryService`:**
   - `view` (grouped data plus `display` strings), `add` (normalise and match the text, then take the default store and aisle; a client id makes it idempotent), `check`/`uncheck` (whatever the version), `low`, `update`, `delete`;
   - `generate` (§4.5: scaling, staples section, merge rules, idempotent sources, `meals_change`);
   - `reconcile` (recompute the given entries' contributions from the current plan; deleted entries contribute zero; manual and checked items untouched; `meals_change`);
   - `undo` (apply `inverse`; 10-minute expiry; by the same member or device, or any adult).
7. **Jobs and schedules:**
   - `recipe.image`: fetch with the same policy, at most 5 MB, sniff png, jpeg or webp, decode, resize to at most 800 px on the long side, encode WebP, store if 512 KB or less; logs carry only ids.
   - `ingredient.llm`: send the cleaned line and the 5 nearest names, using schema `IngredientLlmResult`; store `proposal` and set `match_state = proposed`; never write `ingredient` rows.
   - `grocery.archive` (`*/15 * * * *`): archive checked items older than `grocery_archive_hours`, and prune `meals_change`.
8. **`ensure_defaults(conn)`** (idempotent, household from the tx context): `meals_settings` row, aisles, ingredients (`origin = seed`, skipping existing canonical names), the four slots (only Dinner visible), and the grocery list. Return how many rows were inserted.
9. **Events:** `ChangeBus::publish` `mealplan.updated`, `grocery.updated` or `recipes.updated` (payload `{}`) inside each writing transaction.

## Tests (`crates/fa-meals/tests/`, `#[sqlx::test]` or the fresh container on port 55405, fictional data)
- **Normaliser table tests:** at least 40 lines, covering fractions, ranges, `1 (14 oz) can`, plurals and the exceptions, descriptors, alias and trigram matches, conversion and both display systems, and that volume and weight never merge.
- **Generation:** merge across two recipes; scaling 4 → 6 servings; staples go to the section; running twice adds nothing; hidden slots are skipped; undo restores.
- **Reconcile:** a deleted entry reduces and removes items; checked and manual items are untouched.
- **Templates:** preview lists conflicts; apply with `keep` preserves a free-text entry; undo; editing the template leaves applied entries unchanged.
- **The list:** a repeated client-id add; check-off on a stale version still checks; `low`; archive.
- **Jobs:** `ingredient.llm` against an axum stub stores a proposal and writes no ingredient; with no client it is a no-op. `recipe.image` with a generated in-memory PNG served by a stub; a private host is refused when `allow_private_hosts` is false.
- `ensure_defaults` twice gives the same counts.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-meals` passes against a fresh Postgres.
- [ ] `cargo clippy -p fa-meals --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes. T-03 runs in parallel; if the workspace is red only inside `fa-recipe`, say so.
- [ ] The report lists:
  - the public API with signatures (T-06 builds on it);
  - the rounding table as shipped;
  - deviations from `arch/Meals.md`;
  - "list what outside your paths needs changing";
  - README changes needed: none.
