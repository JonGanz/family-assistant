---
id: T-04
title: "Ingredient and aisle seed data (hand-written, AGPL)"
milestone: M5
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-meals/seed/**", "crates/fa-meals/tests/seed_data.rs"]
decisions: [D-114, D-021, D-113, D-115, D-035]
---

# T-04: Ingredient and aisle seed data

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M5 adds recipes, a meal plan and one living grocery list. The grocery list needs an ingredient catalogue: canonical names, aliases and aisles. Ingredient lines are matched against it, staples are flagged in it, and items are grouped by its aisles. Your job is the **data** that every new household starts with. The loader that inserts it is T-05's.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: *Seed data format*.
- `decisions/D-114-ingredient-seed.md`, `decisions/D-021-living-grocery-list.md`.
- `research/R-007-ingredient-normalisation.md`: sections 2 to 5 (seed contents, units, aisle taxonomies, recommendation).
- `crates/fa-recipe/src/ingredient.rs`: the unit list the parser knows.

## You own
`crates/fa-meals/seed/**` and `crates/fa-meals/tests/seed_data.rs`. The crate skeleton (`Cargo.toml`, `src/lib.rs`) is the main session's until T-05 takes it. `src/lib.rs` already has `pub mod seed_data;` with `include_str!` of your files and the serde types from *Seed data format*. If they are missing, report it and do not add them.

## Work
1. Write `crates/fa-meals/seed/aisles.json`: about 14 aisles in a sensible walking order, for example Produce, Bakery, Meat & Seafood, Deli, Dairy & Eggs, Frozen, Pantry, Canned & Jarred, Baking, Spices & Seasonings, Breakfast & Cereal, Snacks, Beverages, Household. **No "Other" entry:** an item without an aisle shows as "Other" (a null aisle).
2. Write `crates/fa-meals/seed/ingredients.json`: **about 300** common household ingredients, written by you for this project.
   - **Licence (D-114):** copy no third-party list. Don't derive the file from Open Food Facts, USDA or any website's taxonomy. Use your own general knowledge of common foods only.
   - Each entry: `canonical_name` (singular, lower case, ASCII, the most common household name: "tomato", "all-purpose flour", "olive oil"), `aliases` (plurals, common variants and spellings: "tomatoes", "roma tomato"; never the canonical name itself), `aisle` (an exact name from `aisles.json`), `default_unit` (`"g"`, `"ml"`, `"count"`, or null), and `staple: false` everywhere (the household picks its staples).
   - Cover produce, dairy, meat, fish, grains and pasta, canned goods, baking, spices and herbs, oils and condiments, frozen, breakfast, snacks and beverages, plus a few household items (paper towels, dish soap), so manual list items group well.
   - Descriptor words ("kosher", "fresh", "large") are stripped by the normaliser, so don't create entries that differ only by them. "Salt" is one entry, and "kosher salt" is an alias only if you judge it common.
3. Write `crates/fa-meals/seed/descriptors.json`: the default descriptor stop list (R-007), about 40 words such as fresh, large, small, medium, chopped, diced, minced, sliced, kosher, organic, boneless, skinless, ripe, finely, roughly, divided, softened, melted, room-temperature.
4. Write `crates/fa-meals/seed/singular_exceptions.json`: words `pluralizer` gets wrong or that must not change ("hummus", "couscous", "molasses", "asparagus", "swiss", "brussels sprout" ↔ "brussels sprouts", "chickpea"). Each entry is a `{ "plural": ..., "singular": ... }` pair, or `{ "keep": ... }`.
5. `crates/fa-meals/tests/seed_data.rs` (uses only `fa_meals::seed_data`):
   - every file parses;
   - canonical names are unique and lower case;
   - no alias equals any canonical name, and no alias appears twice anywhere;
   - every `aisle` exists in `aisles.json`, and no aisle is named "Other";
   - the count is between 250 and 400;
   - every `default_unit` is one of `g`, `ml`, `count` or null;
   - no entry contains an e-mail, URL or digit.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-meals --test seed_data` passes. Other `fa-meals` tests may not exist yet.
- [ ] `cargo fmt --check -p fa-meals` passes.
- [ ] The report gives the entry count per aisle and confirms that no third-party list was used. It says "list what outside your paths needs changing" (normally nothing). README changes needed: none.
