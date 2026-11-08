---
id: T-06
title: "fa-api meals routes, dashboard meals, wiring and seeding hook"
milestone: M5
wave: 3
depends_on: [T-05, T-03]
agent: {model: sonnet, effort: medium}
status: todo
owns: ["crates/fa-api/**", "bins/fa-server/**"]
decisions: [D-109, D-110, D-112, D-116, D-117, D-118, D-015, D-019]
---

# T-06: Meals API and wiring

You are a sub-agent implementing one task of milestone M5 of the Family Assistant project, a self-hosted family organiser with a Rust backend and a Vue frontend. M5 adds recipes, a meal plan with templates, and one living grocery list. The behaviour is in `fa-meals` (T-05). You expose it over HTTP with authorization, add today's meals to the dashboard payload, and wire the service, its jobs and the per-household seeding into the server.

## Read first
- `plan/_rules-common.md` and `plan/M5/_rules.md`.
- `plan/M5.md`: the *API* table, *API (T-06): notes* and *Domain types* → Authz.
- `arch/Meals.md` §5 and §6.
- T-05's report and `crates/fa-meals/src/lib.rs`.
- Patterns:
  - `crates/fa-api/src/tasks.rs` (handlers, `require`, `parse_version`, `with_etag`, `From<TaskError>` in `error.rs`);
  - `crates/fa-api/src/feeds.rs` (`put_avatar`, `get_avatar`, `get_dashboard`);
  - `crates/fa-api/src/ai.rs` `extract_recipe` (the alias);
  - `crates/fa-api/src/lib.rs` (router, `Services`, `AppState`) and `openapi.rs`;
  - `crates/fa-api/src/setup.rs` `setup_household`;
  - `bins/fa-server/src/wiring.rs` (`feature_handlers`, `ensure_feature_schedules`, `seed_default_rules`, building `Services` and the shared `fa_llm::Provider`).

## You own
`crates/fa-api/**` and `bins/fa-server/**`.

## Work
1. **`crates/fa-api/src/meals.rs`:** one handler per *API* row, each with `#[utoipa::path]` (tag `meals`, `security(cookie, bearer)`, responses including `Problem`), registered in `openapi.rs` (paths and schemas) and in the router under `/api/v1`.
   - Authz: `require(actor, Action::X, &Resource::Meals | GroceryItem)` per *Domain types* → Authz.
   - Versions: `parse_version` from `If-Match`; responses carry an `ETag`.
2. **Errors:** `impl From<fa_meals::MealsError> for ApiError` with the codes from *API (T-06): notes*.
3. **Images:** `GET/POST/DELETE /recipes/{id}/image`, following the avatar routes: a 5 MB `DefaultBodyLimit` on the upload, Content-Type checked, sniffed; `ETag` and `304`; `Cache-Control: private, max-age=86400`; `nosniff`.
4. **Import:** `POST /recipes/import` calls `MealsService::import`. Make `POST /ai/recipes/extract` call the same code (keep its request shape `{url}|{html}`: `html` goes through `fa_recipe::extract` with `RecipeInput::Html`) and keep its tests green. Both now honour `allow_private_hosts` from the runtime config, as the calendar does.
5. **Dashboard:** `get_dashboard` fills `meals` with today's entries (household tz) in visible slots, in slot order: `DashboardMeal{slot_name, time "HH:MM", title, recipe_id}`.
6. **`/meals/settings` PUT** writes `meal_plan_horizon_weeks` through `HouseholdRepo::set_meal_plan_horizon` (1 to 52) and publishes `household.updated`.
7. **SSE doc:** add `mealplan.updated`, `grocery.updated` and `recipes.updated` to the `routes::stream` doc string.
8. **Wiring:**
   - add `meals: Arc<fa_meals::MealsService>` to `Services`, built with the shared LLM `Provider` and the runtime `allow_private_hosts`;
   - add `fa_meals::handlers` and `default_schedules` to `feature_handlers` and `ensure_feature_schedules`;
   - call `fa_meals::ensure_defaults` next to `ensure_default_rules` in `setup_household` (best effort, logged) and in `wiring::seed_default_rules`.
9. **Tests** (`crates/fa-api/tests/m5.rs`, fresh Postgres on port 55406, fictional data):
   - an authz matrix sample: a child adds and checks but cannot delete (403); a device adds without a PIN; an elevated device deletes; a child rates their own;
   - `If-Match` 428 and 409;
   - import of a local fixture page served by an axum stub, with `allow_private_hosts` on;
   - the template preview/apply flow;
   - generate, then undo;
   - a plan delete returns `grocery_impact`, and reconcile applies it;
   - the dashboard carries `meals`;
   - setup seeds the catalogue;
   - the image upload round trip with a generated PNG and a 304.

## Acceptance
- [ ] `df -h /` shows at least 6 GB free; `CARGO_INCREMENTAL=0`.
- [ ] `cargo test -p fa-api -p fa-server` passes against a fresh Postgres.
- [ ] `cargo clippy -p fa-api -p fa-server --all-targets -- -D warnings` and `cargo fmt --check` pass.
- [ ] `cargo run -q -p fa-server -- gen-openapi --out /tmp/claude-1000/m5-openapi.json` succeeds (a scratch path: the main session regenerates `apps/web/src/gen`).
- [ ] The report lists:
  - every route with its handler name;
  - the error codes;
  - "list what outside your paths needs changing";
  - README changes needed (expected: none).
