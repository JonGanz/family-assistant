//! `ensure_defaults`: the per-household starting data (D-114). Idempotent: it runs at setup and
//! at every start, so each part is seeded only while its table is untouched. A household that
//! renamed or deleted a seed aisle, slot or ingredient is never given it back.

use chrono::NaiveTime;
use fa_domain::{BaseUnit, HouseholdId, IngredientOrigin};
use fa_store::{
    AisleRepo, GroceryRepo, IngredientRepo, MealSlotRepo, MealsSettingsRepo, NewIngredient,
};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{MealsError, Result};
use crate::seed_data;

/// Breakfast, Lunch, Snack and Dinner; only Dinner is shown (D-111).
const SLOTS: [(&str, u32, u32, bool); 4] = [
    ("Breakfast", 7, 30, false),
    ("Lunch", 12, 0, false),
    ("Snack", 15, 30, false),
    ("Dinner", 18, 0, true),
];

async fn exists(conn: &mut PgConnection, sql: &str) -> Result<bool> {
    Ok(sqlx::query_scalar(sql).fetch_one(conn).await?)
}

/// Seeds the settings row, aisles, the four slots, the ingredient catalogue and the grocery
/// list for the transaction's household (`tx_for`). Returns how many rows were inserted.
pub async fn ensure_defaults(conn: &mut PgConnection) -> Result<usize> {
    let household: Option<Uuid> = sqlx::query_scalar("SELECT app_household()")
        .fetch_one(&mut *conn)
        .await?;
    let household = HouseholdId::from(
        household
            .ok_or_else(|| MealsError::invalid("household", "no household in this transaction"))?,
    );
    let mut n = 0usize;

    if !exists(conn, "SELECT EXISTS (SELECT 1 FROM meals_settings)").await? {
        MealsSettingsRepo::get_or_default(conn, household).await?;
        n += 1;
    }

    if AisleRepo::count(conn).await? == 0 {
        let rows: Vec<_> = seed_data::aisles()
            .into_iter()
            .map(|a| (None, a.name))
            .collect();
        n += AisleRepo::replace(conn, household, &rows).await?.len();
    }

    if MealSlotRepo::count(conn).await? == 0 {
        for (i, (name, hh, mm, visible)) in SLOTS.iter().enumerate() {
            let time = NaiveTime::from_hms_opt(*hh, *mm, 0)
                .ok_or_else(|| MealsError::invalid("time", "bad seed slot time"))?;
            let ord = i32::try_from(i).unwrap_or(0);
            MealSlotRepo::insert(conn, household, name, ord, time, *visible).await?;
            n += 1;
        }
    }

    if !exists(
        conn,
        "SELECT EXISTS (SELECT 1 FROM ingredient WHERE origin = 'seed')",
    )
    .await?
    {
        let aisles: std::collections::HashMap<String, fa_domain::AisleId> = AisleRepo::list(conn)
            .await?
            .into_iter()
            .map(|a| (a.name.to_lowercase(), a.id))
            .collect();
        let items: Vec<NewIngredient> = seed_data::ingredients()
            .into_iter()
            .map(|i| NewIngredient {
                canonical_name: i.canonical_name,
                aliases: i.aliases,
                is_staple: i.staple,
                aisle_id: aisles.get(&i.aisle.to_lowercase()).copied(),
                default_store_id: None,
                default_unit: i
                    .default_unit
                    .as_deref()
                    .and_then(|u| u.parse::<BaseUnit>().ok()),
                origin: IngredientOrigin::Seed,
            })
            .collect();
        n += usize::try_from(IngredientRepo::insert_many(conn, household, &items).await?)
            .unwrap_or(0);
    }

    if !exists(conn, "SELECT EXISTS (SELECT 1 FROM grocery_list)").await? {
        GroceryRepo::list_id(conn, household).await?;
        n += 1;
    }
    Ok(n)
}
