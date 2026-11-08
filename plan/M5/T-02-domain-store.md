---
id: T-02
title: "Domain and store: meals types, recipe draft move, authz, migration, repos"
milestone: M5
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: done
owns: ["crates/fa-domain/**", "crates/fa-store/**", "crates/fa-recipe/src/types.rs", "crates/fa-recipe/Cargo.toml", "crates/fa-api/src/feeds.rs"]
decisions: [D-109, D-110, D-111, D-112, D-113, D-115, D-116, D-118, D-120, D-121, D-122, D-021, D-015]
---

# T-02: Domain and store for meals

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Rust backend (Postgres with row-level security per household) and a Vue frontend. M5 adds recipes, a meal plan with templates, and one living grocery list grouped by store and aisle. Every other M5 crate depends on your types and repos, so you do both `fa-domain` and `fa-store` in one task (CLAUDE.md).

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`, *Contracts fixed up front*: *Database*, *Domain types*, *Store*, and the *API* table (it names every DTO field).
- `arch/Meals.md` §3 (tables and invariants) and §6 (authorization); `arch/Domain.md`, the Meals package and RBAC rows.
- Code to copy:
  - `crates/fa-store/src/repo/task_list.rs` (repo shape, `Versioned`, version check with `conflict_or_missing`);
  - `crates/fa-store/migrations/20261104000001_m4_llm.up.sql` (the RLS `DO $$` block);
  - `crates/fa-domain/src/entities.rs` (`db_enum!`), `ids.rs` (`id_type!`), `tasks_api.rs` (`dto!`, `TaskPage`);
  - `crates/fa-domain/src/authz.rs` (`Action`, `Action::ALL`, `Resource`, `authorize`, the test matrix around `:928` and `all_lists_every_variant`);
  - `crates/fa-domain/src/api.rs` (`DashboardView`);
  - `crates/fa-recipe/src/types.rs`.

## You own
- `crates/fa-domain/**` and `crates/fa-store/**`.
- `crates/fa-recipe/src/types.rs` and `crates/fa-recipe/Cargo.toml` (the move only).
- In `crates/fa-api/src/feeds.rs`, only the `DashboardView { … }` literal, where you add `meals: Vec::new()`.

## Work
1. **Migration** `crates/fa-store/migrations/20261108000001_m5_meals.up.sql` and `.down.sql`, exactly as the *Database* table says:
   - `CREATE EXTENSION IF NOT EXISTS pg_trgm` first;
   - every table, CHECK, UNIQUE and index; the GIN trigram indexes on `recipe.title` and `ingredient.canonical_name`; the GIN index on `ingredient.aliases`;
   - one RLS `DO $$` block listing every new table.
   The down migration drops the tables in reverse dependency order, but not the extension.
2. **`fa-domain`:**
   - Move `RecipeDraft`, `RecipeIngredient` and `RecipeOrigin` to `src/recipe.rs`. Add `RecipeOrigin::Microdata`. Keep the ts export path.
   - Leave `fa-recipe/src/types.rs` re-exporting them, with `RecipeError` and `RecipeInput` staying there. Add `fa-domain.workspace = true` to `fa-recipe/Cargo.toml`.
   - About `JsonSchema`: grep the workspace for uses of it on these types. If there are none, drop the derive. Otherwise add `schemars.workspace = true` to `fa-domain` and say so.
3. **`fa-domain` types:**
   - The IDs, enums and every DTO from *Domain types*, in `src/meals_api.rs`. Fields and names exactly as listed; nullable fields with `#[ts(optional = nullable)]`.
   - Validation helpers with unit tests:
     - `parse_slot_time(&str) -> Result<NaiveTime, …>`, which accepts only `"HH:MM"`, 00:00 to 23:59;
     - `format_slot_time(NaiveTime) -> String`.
4. **`DashboardSettings`** (`dashboard.rs`) gains `#[serde(default = "default_true")] pub food_view: bool`, set to `true` in its `Default`. Add a serde test: old stored JSON without the field reads as `true`.
5. **`DashboardView`** gains `#[serde(default)] pub meals: Vec<DashboardMeal>`. Add `meals: Vec::new()` at its one construction site in `fa-api/src/feeds.rs` and nowhere else in that file.
6. **Authz:**
   - The five new actions and `Resource::Meals`, with the rules from *Domain types* → Authz.
   - `Action::ALL` grows to 37; update the length assertion and `all_lists_every_variant`.
   - Add a test row per action for Adult, Child, Device, Elevated and System.
   - `AddGroceryItem` stays allowed for everyone.
7. **Repos** (runtime `sqlx::query`/`query_as`, no `cargo sqlx prepare`): every repo listed in *Store*, `TaskListRepo`-shaped. At least:
   - CRUD per table;
   - versioned `update` where the table has `version`;
   - `RecipeLineRepo::replace_for_recipe(conn, recipe, lines)`. It keeps the id and match of a line whose `raw_text` is unchanged;
   - `IngredientRepo::{match_exact, match_alias, nearest, add_alias, merge, insert_many}`. `nearest` orders by `similarity(canonical_name, $1)` desc and returns the score;
   - `MealPlanRepo::{range, get, insert (ord = max + 1 in its date and slot), update, delete, swap, entries_at(date, slot), next_ord}`. `swap` exchanges `(date, slot_id, ord)` of two rows. There is no unique on (date, slot) (D-120);
   - `MealTemplateRepo::{list, get_with_cells, insert, update_with_cells, delete, duplicate}`;
   - `GroceryStoreRepo::{list, replace (a new store gets `grocery_store_aisle` rows copied from `aisle.ord`), aisles_for(store) (every aisle in that store's order, missing rows falling back to `aisle.ord`, not hidden), set_aisles}` (D-121);
   - `GroceryRepo::{list_id, items_open, insert_item (with an optional client id; ON CONFLICT (id) DO NOTHING plus a returned flag), check, uncheck, set_section, update_item, delete_item, sources_for_entries, upsert_source, delete_source, archive_checked_before}`;
   - `MealsChangeRepo::{insert, get, mark_undone, prune_before}`;
   - `MealsSettingsRepo::{get_or_default, update}`;
   - `HouseholdRepo::set_meal_plan_horizon`.
8. **Tests** (`crates/fa-store/tests/m5.rs`, `#[sqlx::test]`, fictional data):
   - the migration applies and reverts;
   - RLS hides another household's recipe, item and entry;
   - several entries in one slot keep their `ord`;
   - `aisles_for` falls back to the household order for a missing row and honours `hidden`;
   - `swap`;
   - `insert_item` with a repeated client id is idempotent;
   - the version conflict;
   - `nearest` ranks "tomatoe" closest to "tomato";
   - an alias match;
   - `merge` re-points lines and items;
   - `archive_checked_before`.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] Fresh Postgres on port 55402. `SQLX_OFFLINE=true DATABASE_URL=… cargo test -p fa-domain -p fa-store` passes.
- [ ] `cargo clippy -p fa-domain -p fa-store -p fa-recipe --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `SQLX_OFFLINE=true cargo check --workspace` passes. The workspace stays green: no "red until the next wave".
- [ ] The report lists:
  - every repo fn signature, which T-05 builds on;
  - the DTO names;
  - the authz rows;
  - whether `JsonSchema` was kept;
  - "list what outside your paths needs changing";
  - README changes needed: none.

## Outcome (T-02 report, 2026-11-08)
- **Built:**
  - migration `20261108000001_m5_meals` with `pg_trgm`, 18 tables, one RLS block, and `set_updated_at` triggers;
  - in `fa-domain`: `recipe.rs` (the moved draft types plus `Microdata`), `meals_api.rs` (every DTO, `parse_slot_time` and `format_slot_time`), 10 ids, 6 enums, `DashboardSettings.food_view`, `DashboardView.meals`, and the five authz actions with `Resource::Meals` (`Action::ALL` = 37);
  - in `fa-store`: `repo/meals_recipe.rs`, `meals_plan.rs`, `meals_grocery.rs`, `HouseholdRepo::set_meal_plan_horizon`, and `tests/m5.rs` (16 tests).
- **Results** (reported, fresh Postgres): fa-domain 359 passed; fa-store 94 passed; clippy and fmt clean; `cargo check --workspace` clean.
- **Deviations:**
  - `RecipeRepo::delete` writes the title into the entries' `free_text` first, because the xor CHECK forbids the FK's SET NULL alone.
  - `MealSlotRepo::replace` returns `SlotReplace::InUse(id)` rather than a store error.
  - `JsonSchema` was dropped from the moved types; `fa-domain` needs no `schemars`.
  - The repos return `*Row` types; patch structs are named `*Edit` to avoid clashing with the `fa_domain` DTOs.
- **For T-05 and T-06:**
  - `fa_domain::RecipeInput` (the DTO) and `fa_recipe::RecipeInput` (the enum) coexist; import them by path.
  - The archive job loops households with `tx_for`.
  - The updates return `StoreError::Conflict` or `NotFound`.
  - The repo signatures are below; the code is the final word.

### Repo signatures (`fa_store`; every fn takes `conn: &mut PgConnection` first)
- **`RecipeRepo`:**
  - `insert(id, household, &RecipeFields, ImportMethod, created_by)` → `RecipeRow`; `get`; `update(id, &RecipeFields, expected_version)` → i32; `set_favourite`; `delete` (entries become free text);
  - `list(&RecipeQuery{q, tag, favourite, me, today, after: Option<(String, RecipeId)>, limit})` → `Vec<RecipeListRow>`; `all_tags`; `count`.
- **`RecipeImageRepo`:** `put(recipe, mime, bytes, sha256, source)`, `get`, `sha`, `delete`.
- **`RecipeLineRepo`:** `for_recipe`, `get`, `replace_for_recipe(recipe, &[NewLine])` → `Vec<ReplacedLine{row, kept}>`, `set_match(id, ingredient, MatchState, proposal)`, `unmatched(limit)`.
- **`RecipeStepRepo`:** `for_recipe`, `replace_for_recipe`.
- **`RecipeRatingRepo`:** `set(recipe, member, stars)`, `clear`, `for_recipe`.
- **`IngredientRepo`:** `insert`, `insert_many(household, &[NewIngredient])` (skips existing), `get`, `list`, `search(q, threshold, limit)`, `match_exact`, `match_alias`, `nearest(name, limit)` → `Vec<(IngredientId, String, f32)>`, `add_alias`, `update`, `set_staple`, `delete`, `merge(from, into)`, `count`.
- **`AisleRepo`:** `list`, `count`, `replace(household, &[(Option<AisleId>, String)])`.
- **`GroceryStoreRepo`:** `list`, `replace(household, &[(Option<GroceryStoreId>, String)])` (a new store copies the aisle order), `aisles_for(store)` → `Vec<StoreAisleRow{aisle_id, ord, hidden}>`, `set_aisles(store, &[(AisleId, hidden)])`.
- **`MealSlotRepo`:** `list`, `get`, `count`, `insert(household, name, ord, NaiveTime, visible)`, `in_use`, `replace(household, &[SlotInput])` → `SlotReplace::{Done(rows), InUse(id)}`.
- **`MealPlanRepo`:** `range(from, to)` (inclusive), `get`, `many`, `entries_at(date, slot)`, `next_ord`, `insert(household, &NewEntry{date, slot_id, content: EntryContent::{Recipe, FreeText}, servings, created_by})`, `restore`, `update(id, &EntryPatch, expected_version)` (a new date or slot appends), `delete`, `delete_at(date, slot)` → deleted rows, `swap(a, b)`, `day_meals(date)` → `Vec<DayMealRow{slot_name, slot_time, title, recipe_id}>`.
- **`MealTemplateRepo`:** `list` → `Vec<MealTemplateListRow{row, slot_ids}>`, `get`, `cells`, `get_with_cells`, `insert(household, &NewTemplate, &[NewTemplateCell{day, slot_id, ord, content, servings}])`, `update_with_cells(id, t, cells, expected_version)`, `delete`, `duplicate(id, name)`.
- **`GroceryRepo`:**
  - `list_id(household)` (lazy), `items_open(list)`, `get_item`, `insert_item(household, list, &NewGroceryItem{id: Option, …})` → `(row, created)`, `check(id, by)`, `uncheck`, `set_section` (also unchecks), `update_item(id, &GroceryItemEdit, expected_version)`, `set_quantities(id, qty, unit, qty2, unit2)`, `delete_item`;
  - `sources_for_entries`, `hints_for_items` → `SourceHintRow{item_id, entry_id, date, slot_name}`, `upsert_source(household, &GrocerySourceRow)`, `delete_source(item, entry)`, `archive_checked_before(cutoff)`.
- **`MealsChangeRepo`:** `insert(household, kind, actor_member, actor_device, &inverse, expires_at)`, `get`, `mark_undone`, `prune_before`.
- **`MealsSettingsRepo`:** `get_or_default(household)`, `update(household, &MealsSettingsPatchRow)`.
- **`HouseholdRepo`:** `set_meal_plan_horizon(id, weeks)`.
