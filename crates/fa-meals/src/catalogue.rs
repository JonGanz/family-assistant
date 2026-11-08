//! The ingredient catalogue (Settings → Meals and the add-box suggestions).

use fa_domain::*;
use fa_store::{AisleRepo, GroceryStoreRepo, IngredientRepo, IngredientRow, NewIngredient};
use sqlx::PgConnection;

use crate::error::{MealsError, Result};
use crate::service::{MealsService, publish};

fn view(r: IngredientRow) -> IngredientView {
    IngredientView {
        id: r.id,
        canonical_name: r.canonical_name,
        aliases: r.aliases,
        is_staple: r.is_staple,
        aisle_id: r.aisle_id,
        default_store_id: r.default_store_id,
        default_unit: r.default_unit,
        origin: r.origin,
    }
}

async fn checked(conn: &mut PgConnection, i: &IngredientInput) -> Result<NewIngredient> {
    let name = i.canonical_name.trim().to_lowercase();
    if name.is_empty() || name.chars().count() > 100 {
        return Err(MealsError::invalid("canonical_name", "1 to 100 characters"));
    }
    let mut aliases: Vec<String> = Vec::new();
    for a in &i.aliases {
        let a = a.trim();
        if a.is_empty() || a.eq_ignore_ascii_case(&name) {
            continue;
        }
        if a.chars().count() > 100 {
            return Err(MealsError::invalid(
                "aliases",
                "an alias has at most 100 characters",
            ));
        }
        if !aliases.iter().any(|x| x.eq_ignore_ascii_case(a)) {
            aliases.push(a.to_string());
        }
    }
    if let Some(a) = i.aisle_id
        && AisleRepo::list(conn).await?.iter().all(|x| x.id != a)
    {
        return Err(MealsError::invalid("aisle_id", "unknown aisle"));
    }
    if let Some(s) = i.default_store_id
        && GroceryStoreRepo::list(conn)
            .await?
            .iter()
            .all(|x| x.id != s)
    {
        return Err(MealsError::invalid("default_store_id", "unknown store"));
    }
    Ok(NewIngredient {
        canonical_name: name,
        aliases,
        is_staple: i.is_staple,
        aisle_id: i.aisle_id,
        default_store_id: i.default_store_id,
        default_unit: i.default_unit,
        origin: IngredientOrigin::User,
    })
}

impl MealsService {
    /// The whole catalogue, or the best matches of `q` (name or alias contains it, or is similar).
    pub async fn list_ingredients(
        &self,
        h: HouseholdId,
        q: Option<&str>,
    ) -> Result<Vec<IngredientView>> {
        let mut tx = self.store.tx_for(h).await?;
        let rows = match q.map(str::trim).filter(|s| !s.is_empty()) {
            Some(q) => {
                let (s, _) = self.settings_and_normaliser(&mut tx, h).await?;
                IngredientRepo::search(&mut tx, q, s.trigram_threshold, 50).await?
            }
            None => IngredientRepo::list(&mut tx).await?,
        };
        tx.rollback().await?;
        Ok(rows.into_iter().map(view).collect())
    }

    pub async fn create_ingredient(
        &self,
        h: HouseholdId,
        input: IngredientInput,
    ) -> Result<IngredientView> {
        let mut tx = self.store.tx_for(h).await?;
        let n = checked(&mut tx, &input).await?;
        let row = IngredientRepo::insert(&mut tx, h, &n).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(view(row))
    }

    pub async fn update_ingredient(
        &self,
        h: HouseholdId,
        id: IngredientId,
        input: IngredientInput,
    ) -> Result<IngredientView> {
        let mut tx = self.store.tx_for(h).await?;
        let n = checked(&mut tx, &input).await?;
        let row = IngredientRepo::update(&mut tx, id, &n).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(view(row))
    }

    /// The staple flag on its own (instant save).
    pub async fn set_ingredient_staple(
        &self,
        h: HouseholdId,
        id: IngredientId,
        on: bool,
    ) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        IngredientRepo::set_staple(&mut tx, id, on).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Lines and items lose the link.
    pub async fn delete_ingredient(&self, h: HouseholdId, id: IngredientId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        IngredientRepo::delete(&mut tx, id).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Re-points lines and items to `into` and adds the name and aliases of `id` as aliases.
    pub async fn merge_ingredient(
        &self,
        h: HouseholdId,
        id: IngredientId,
        into: IngredientId,
    ) -> Result<IngredientView> {
        if id == into {
            return Err(MealsError::invalid(
                "into",
                "cannot merge an ingredient into itself",
            ));
        }
        let mut tx = self.store.tx_for(h).await?;
        IngredientRepo::merge(&mut tx, id, into).await?;
        let row = IngredientRepo::get(&mut tx, into)
            .await?
            .ok_or(MealsError::NotFound)?;
        publish(&mut tx, h, "recipes.updated").await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(view(row))
    }
}
