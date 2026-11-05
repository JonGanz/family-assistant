---
id: T-07
title: "fa-recipe: JSON-LD first, LLM fallback, RecipeDraft"
milestone: M4
wave: 2
depends_on: [T-04]
agent: {model: sonnet, effort: medium}
status: done
owns: ["crates/fa-recipe/**"]
decisions: [D-094, D-093, D-015]
---

# T-07: `fa-recipe`: JSON-LD first, LLM fallback, `RecipeDraft`

You are a sub-agent implementing one task of milestone M4 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M4 clears the way for M5 (meal planning and recipes) with a recipe-extraction contract: deterministic schema.org parsing first, and the LLM only for pages without structured data. This crate stores nothing; M5 adds storage.

## Read first
- `plan/_rules-common.md` and `plan/M4/_rules.md`.
- `plan/M4.md`: *`fa-recipe`* and the `POST /ai/recipes/extract` row of *API*.
- `arch/AI.md` §9; `arch/Domain.md` (`Recipe`, `import_method`).
- `crates/fa-connectors/src/http.rs`: `Http::with_policy(allow_http, allow_private_hosts)`, `parse_url`, `get` and `send`. It blocks private targets at connect time and on every redirect, and caps the body (`MAX_BODY`, 10 MB). **Use it for the fetch** instead of a hand-rolled client. The plan's "5 MB cap, 15 s timeout, at most 3 redirects" give way to this shared client's policy; report what it enforces.
- `crates/fa-rules/src/` for how `scraper` JSON-LD extraction is already done (the schema.org events extractor), so you reuse the approach.
- The `fa-llm` public API (T-04): `LlmClient::extract`, `ExtractRequest`, `LlmError`.

## You own
`crates/fa-recipe/**` only. The main session added the workspace member. If a dependency is missing from the root `[workspace.dependencies]`, report it.

## Work
1. `Cargo.toml` (`workspace = true`): `fa-llm`, `fa-connectors`, `scraper`, `serde`, `serde_json`, `schemars`, `thiserror`, `url`, `ts-rs`, `utoipa`, `tracing`. Check how `fa-domain` exports ts types (the `export_to` path) and do the same, so that `RecipeDraft` and `RecipeIngredient` reach `apps/web/src/gen/types/`.
2. `RecipeDraft` and `RecipeIngredient`, exactly as the plan's field list (`origin` is a `RecipeOrigin { Jsonld => "jsonld", Llm => "llm" }`), with `Serialize`, `Deserialize`, `JsonSchema`, `TS` and `ToSchema`. Nullable fields follow the common ts rule (`#[ts(optional = nullable)]`).
3. `from_html(html, source_url) -> Option<RecipeDraft>`:
   - Read every `script[type="application/ld+json"]`. Accept an object, an array or `@graph`, and `@type` either `"Recipe"` or an array containing it.
   - Map `name`; `recipeYield` (a string or number or array) to `yield_text`, plus `servings` when a leading integer is found; `prepTime`, `cookTime` and `totalTime` (ISO 8601 → minutes); `recipeIngredient`; `recipeInstructions` (a string, `HowToStep[]` or `HowToSection[].itemListElement`); `keywords` and `recipeCategory`/`recipeCuisine` to `tags` (split on commas, trimmed, deduplicated); and `image` (a string, an object's `url`, or the first of an array).
   - Use the first recipe found, or the one with the most ingredients.
4. `parse_iso_duration(&str) -> Option<u32>`, giving minutes: handles `PT1H30M`, `PT45M`, `P0DT1H`, and `PT90S` (rounded up). Anything else is `None`.
5. `parse_ingredient(line) -> RecipeIngredient`: `text` is always the line. Then a rule parser:
   - quantity from integers, decimals, `1/2`, `1 1/2`, unicode fractions (`½ ¼ ¾ ⅓ ⅔ ⅛`) and `1½`, with ranges `2-3` and `2 to 3` taking the low end;
   - unit from a fixed list (`cup(s)`, `tbsp`, `tablespoon(s)`, `tsp`, `teaspoon(s)`, `g`, `kg`, `ml`, `l`, `oz`, `lb(s)`, `pound(s)`, `clove(s)`, `can(s)`, `pinch`, `slice(s)`), normalised to the singular short form;
   - `item` is the rest, trimmed of a leading "of".
   When unsure, the fields are `None`. This is a deterministic first pass; M5 may refine it.
6. `from_llm(client: &LlmClient, text: &str, source_url) -> Result<RecipeDraft, LlmError>`: an `ExtractRequest` with a short system prompt ("extract one recipe; return only what is on the page; empty arrays when absent") and a schema-friendly subset struct (`name`, `yield_text`, `servings`, minutes, `ingredients: [String]`, `steps`, `tags`). Then run `parse_ingredient` on every ingredient line, and set `origin = Llm` and `source_url`. An empty `name` or no ingredients gives `Err(LlmError::Schema("no recipe"))`.
7. `fetch_page(http: &fa_connectors::http::Http, url: &Url) -> Result<String, RecipeError>`, plus `visible_text(html) -> String` (drop `script`, `style` and `nav`; collapse whitespace) for the LLM fallback. `RecipeError` is a thiserror enum: `Fetch`, `NotHtml`, `Llm(LlmError)`.
8. A convenience `extract(http, llm: Option<&LlmClient>, input: RecipeInput) -> Result<RecipeDraft, RecipeError>`, with `RecipeInput::{Url(Url), Html{html, source_url}}`: JSON-LD first, then the LLM when a client is present, else `RecipeError::NoRecipe` (or `LlmUnavailable` when JSON-LD found nothing and there is no client). T-08 maps these to 422 and 503.

## Tests (`crates/fa-recipe/tests/`, fictional HTML fixtures in `tests/fixtures/`)
- JSON-LD: a plain object, `@graph`, an array `@type`, `HowToSection` steps, and an `image` object. Durations and yields.
- A page without JSON-LD returns `None`.
- Ingredients: `1 ½ cups flour`, `2-3 cloves garlic`, `1/4 tsp salt`, `salt to taste` (quantity and unit `None`), and `200 g of butter`.
- `from_llm` against an axum stub that returns the subset JSON: the result carries `origin = llm` and parsed ingredients.
- `fetch_page` refuses `http://127.0.0.1/…` (a private target).

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-recipe`, `cargo clippy -p fa-recipe --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check -p fa-recipe` passes. The workspace may be red from T-06's in-progress `MailDeps` change; say so if it is.
- [ ] The report lists:
  - the public API;
  - the fetch policy actually enforced (from `Http`);
  - the ts export path;
  - "list what outside your paths needs changing";
  - README changes needed: none.

## Outcome (T-07 report)
- **API:**
  - `from_html`, `parse_iso_duration`, `parse_ingredient`, `from_llm`, `fetch_page`, `visible_text` and `extract(http, Option<&LlmClient>, RecipeInput)`.
  - Types `RecipeDraft`, `RecipeIngredient`, `RecipeOrigin` and `RecipeInput`, exported to `gen/types`.
  - `RecipeError::{Fetch, NotHtml, NoRecipe, LlmUnavailable, Llm}`.
  - 12 tests with fictional fixtures.
- **Fetch:** through `Http::with_policy(true, false)`, which blocks private hosts (redirects included). The limits are 10 MB, 20 s and 5 redirects, replacing the plan's 5 MB, 15 s and 3.
- **Deviations:**
  - The LLM schema uses all-required fields, with empty values meaning absent.
  - At most 24,000 characters go to the LLM.
  - When a page has several recipes, the one with the most ingredients wins.
- **For T-08:** map `NoRecipe` to 422, `LlmUnavailable` to 503 and other `Llm` errors to 502.
