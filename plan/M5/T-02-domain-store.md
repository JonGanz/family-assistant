---
id: T-02
title: "Domain and store: meals types, recipe draft move, authz, migration, repos"
milestone: M5
wave: 1
depends_on: [T-01]
agent: {model: sonnet, effort: high}
status: todo
owns: ["crates/fa-domain/**", "crates/fa-store/**", "crates/fa-recipe/src/types.rs", "crates/fa-recipe/Cargo.toml", "crates/fa-api/src/feeds.rs"]
decisions: [D-109, D-110, D-111, D-112, D-113, D-115, D-116, D-118, D-021, D-015]
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
   - `MealPlanRepo::{range, get, insert, update, delete, swap, entry_at}`. `swap` exchanges `(date, slot_id)` of two rows in one statement or with a deferred step, without tripping the unique index;
   - `MealTemplateRepo::{list, get_with_cells, insert, update_with_cells, delete}`;
   - `GroceryRepo::{list_id, items_open, insert_item (with an optional client id; ON CONFLICT (id) DO NOTHING plus a returned flag), check, uncheck, set_section, update_item, delete_item, sources_for_entries, upsert_source, delete_source, archive_checked_before}`;
   - `MealsChangeRepo::{insert, get, mark_undone, prune_before}`;
   - `MealsSettingsRepo::{get_or_default, update}`;
   - `HouseholdRepo::set_meal_plan_horizon`.
8. **Tests** (`crates/fa-store/tests/m5.rs`, `#[sqlx::test]`, fictional data):
   - the migration applies and reverts;
   - RLS hides another household's recipe, item and entry;
   - the unique (date, slot) rule;
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
