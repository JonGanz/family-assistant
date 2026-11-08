---
id: T-01
title: "Main session: plan, rules, workspace, regeneration, integration"
milestone: M5
wave: 0
depends_on: []
agent: {model: opus, effort: medium}
status: done
owns: ["plan/**", "Cargo.toml", "crates/fa-meals/Cargo.toml", "crates/fa-meals/src/lib.rs", "crates/fa-meals/src/seed_data.rs", "apps/web/src/gen/**", "apps/web/src/api/food.ts", "apps/web/src/router/index.ts", "apps/web/src/ui/icons.ts"]
decisions: [D-108, D-109, D-110, D-111, D-112, D-113, D-114, D-115, D-116, D-117, D-118, D-119]
---

# T-01: Main session

Planning was done on 2026-11-08 and produced:
- `plan/M5.md`, `plan/M5/_rules.md` and the briefs T-02 to T-13;
- decisions D-108 to D-119 (questions Q-098 to Q-114);
- research R-007;
- `arch/Meals.md` v1, `Frontend.md` v5.2 and `Domain.md` v5.2.

Batch 25 (Q-115 to Q-117) is open. The briefs build its working assumptions.

Integration rows. Tick each one in `plan/M5.md` *Task status* when it is done.
1. **Before wave 1:**
   - Root `Cargo.toml`:
     - member `crates/fa-meals`;
     - `fa-meals = { path = "crates/fa-meals" }`;
     - `pluralizer = "0.6.0"`;
     - `image = { version = "0.25.10", default-features = false, features = ["jpeg", "png", "webp"] }`.
   - Create the `fa-meals` skeleton:
     - `Cargo.toml` with `fa-domain`, `fa-store`, `serde`, `serde_json` and `thiserror` as `workspace = true`;
     - `src/lib.rs` holding `pub mod seed_data;` and `src/seed_data.rs` with the *Seed data format* types and `include_str!` loaders.
   - Write placeholder seed files (`[]`) so the crate compiles before T-04.
   - Run `cargo clean`: `target/` is 20 GB.
   - Run `SQLX_OFFLINE=true cargo check --workspace`.
2. **After wave 1:**
   - Run `cargo check --workspace` and the T-02 tests against a fresh Postgres.
   - Hand `crates/fa-meals/Cargo.toml` and `src/lib.rs` to T-05; it owns them from wave 2.
   - Delete `target/debug/incremental` and run `docker builder prune -f`.
3. **After wave 2:**
   - Run `cargo test -p fa-meals -p fa-recipe` on a fresh Postgres.
   - Free disk.
4. **After wave 3:**
   - Regenerate `apps/web/src/gen` (ts-rs and OpenAPI): `cargo test -q -p fa-domain && cargo run -q -p fa-server -- gen-openapi --out apps/web/src/gen/openapi.json && cd apps/web && corepack pnpm@9 run gen`.
   - Write the **web scaffold** (`plan/M5.md`, *Web*): `api/food.ts`; the Food and `/settings/meals` routes; stub views and a stub `components/food/FoodTabs.vue`; stub stores exporting the contract names (`useMealPlanStore`, `useGroceryStore`, `useRecipesStore`, `useMealsSettingsStore` with `invalidate`); the icon keys, each checked against the installed Material Symbols. The web checks must be green on the scaffold.
   - Run `scripts/ci-local.sh --no-web`.
   - Free disk.
5. **After wave 4:**
   - Run the web checks: lint, typecheck, test, build.
   - Run `scripts/ui-shots.sh --all` and publish a private review artifact of the Food views at 360, 800 and 1280 px and of the kiosk Food view.
6. **After wave 5:** the web checks and kiosk screenshots.
7. **After wave 6:** the closing checklist (`close-milestone` skill).
