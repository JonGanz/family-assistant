---
id: T-03
title: "fa-recipe: schema.org microdata, ingredient parser extensions"
milestone: M5
wave: 2
depends_on: [T-02]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-recipe/src/jsonld.rs", "crates/fa-recipe/src/microdata.rs", "crates/fa-recipe/src/ingredient.rs", "crates/fa-recipe/src/lib.rs", "crates/fa-recipe/src/llm.rs", "crates/fa-recipe/tests/**"]
decisions: [D-109, D-113, D-094, D-035]
---

# T-03: `fa-recipe`: microdata and parser extensions

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M5 stores recipes. M4's `fa-recipe` reads schema.org JSON-LD and falls back to a local LLM. M5 adds schema.org **microdata**, tried after JSON-LD and before the LLM (D-109). It also extends the rule ingredient parser that the grocery normaliser builds on (D-113, R-007). The crate still stores nothing.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: *`fa-recipe` changes* and *Domain types*.
- `arch/Meals.md` §4.1 and §4.2; `arch/AI.md` §9.
- `research/R-007-ingredient-normalisation.md` §1 (microdata: a hand parser over `scraper`, no RDFa) and §3 (units).
- All of `crates/fa-recipe/src/`. T-02 moved `RecipeDraft`, `RecipeIngredient` and `RecipeOrigin` to `fa-domain` and left `types.rs` re-exporting them. `RecipeOrigin` now has `Microdata`.

## You own
The files in `owns`. `crates/fa-recipe/src/types.rs` and `crates/fa-recipe/Cargo.toml` are T-02's (already done); don't edit them.

## Work
1. **Microdata** (`src/microdata.rs`, `pub fn from_microdata(html: &str, source_url: Option<&Url>) -> Option<RecipeDraft>`):
   - Find the element with `itemscope` whose `itemtype` ends in `schema.org/Recipe` (http or https, with or without `www`).
   - Read its `itemprop`s, ignoring nested `itemscope` elements except `HowToStep` and `HowToSection` inside `recipeInstructions`: `name`, `recipeYield`, `prepTime`/`cookTime`/`totalTime` (from `content` or `datetime`, then `parse_iso_duration`), `recipeIngredient` (or the legacy `ingredients`), `recipeInstructions` (the text, or the steps of nested `HowToStep`), `keywords`, `recipeCategory`, `recipeCuisine` and `image` (`src`, `content` or `href`).
   - The value of an itemprop is `content`, then `src`, `href` or `datetime`, then the element's text, whitespace collapsed.
   - Set `origin: RecipeOrigin::Microdata`. Map every field the same way `jsonld.rs` does: share its helpers for yield, servings and tags (move them to a private module if needed).
2. **Order in `extract`:** JSON-LD (`from_html`), then `from_microdata`, then the LLM. `from_html` keeps its signature and its JSON-LD-only meaning. Export `from_microdata`.
3. **Ingredient parser** (`parse_ingredient`, same signature):
   - New units: `fl oz` / `fluid ounce(s)` → `fl oz`; `pint(s)`/`pt` → `pint`; `quart(s)`/`qt` → `quart`; `gallon(s)`/`gal` → `gallon`; `l`/`liter(s)`/`litre(s)` → `l`; `piece(s)`, `stick(s)`, `bunch(es)`, `head(s)`, `sprig(s)` → singular.
   - The `1 (14 oz) can tomatoes` / `2 (15-ounce) cans beans` form: quantity 1 (2), unit `can`, item "tomatoes" ("beans"). The parenthetical goes to a new `note` only if `RecipeIngredient` has one. It does not (T-02 keeps the M4 shape), so drop it from `item`.
   - Trim trailing preparation after a comma from `item` ("onion, diced" → item "onion"). Strip "to taste" and "as needed" (quantity and unit `None`).
   - Unchanged: `text` is always the source line; ranges take the low end; when unsure, the field is `None`.
4. **Pasted text:** no new API. `fa-meals` calls the existing `from_llm(client, text, None)` for pasted text. Confirm it works with `source_url = None`, and add a test.

## Tests (`crates/fa-recipe/tests/`; fictional fixtures in `tests/fixtures/`, written by you)
- Microdata: a flat recipe; a nested `HowToStep` list; `content` attributes for times; an `image` `<img src>`; a page with both JSON-LD and microdata (JSON-LD wins, `origin = jsonld`); a page with neither (`None`).
- `extract` on a microdata-only page returns `origin = microdata` without calling the LLM: pass `None` for the client.
- Parser: `1 (14 oz) can diced tomatoes`, `2 (15-ounce) cans black beans, drained`, `1 pint cherry tomatoes`, `2 fl oz lime juice`, `1 onion, finely chopped`, `salt and pepper to taste`, `1 bunch cilantro`. The M4 cases stay green.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-recipe`, `cargo clippy -p fa-recipe --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes (T-05 runs in parallel; if it is red only inside `fa-meals`, say so).
- [ ] The report lists the public API changes ("`from_microdata`, `extract` order"), the unit list as shipped, and "list what outside your paths needs changing". README changes needed: none.
