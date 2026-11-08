//! Settings → Meals: the per-household settings, slots, aisles, stores and a store's aisle order.

use std::collections::HashSet;

use fa_domain::*;
use fa_store::{
    AisleRepo, GroceryStoreRepo, HouseholdRepo, MealSlotRepo, MealsSettingsPatchRow,
    MealsSettingsRepo, SlotInput, SlotReplace,
};
use sqlx::PgConnection;

use crate::error::{MealsError, Result};
use crate::seed_data;
use crate::service::{MealsService, publish, to_i32, to_u32};

fn name_ok(field: &'static str, n: &str) -> Result<String> {
    let n = n.trim();
    if n.is_empty() || n.chars().count() > 60 {
        return Err(MealsError::invalid(field, "1 to 60 characters"));
    }
    Ok(n.to_string())
}

fn unique_names<'a>(field: &'static str, names: impl Iterator<Item = &'a str>) -> Result<()> {
    let mut seen = HashSet::new();
    for n in names {
        if !seen.insert(n.to_lowercase()) {
            return Err(MealsError::invalid(field, "names must be different"));
        }
    }
    Ok(())
}

async fn settings_view(conn: &mut PgConnection, h: HouseholdId) -> Result<MealsSettingsView> {
    let s = MealsSettingsRepo::get_or_default(conn, h).await?;
    let hh = HouseholdRepo::get(conn, h)
        .await?
        .ok_or(MealsError::NotFound)?
        .value;
    Ok(MealsSettingsView {
        grocery_archive_hours: to_u32(s.grocery_archive_hours),
        ingredient_llm: s.ingredient_llm,
        trigram_threshold: s.trigram_threshold,
        // An empty stored list means the seed list; show the one in force.
        descriptors: if s.descriptors.is_empty() {
            seed_data::descriptors()
        } else {
            s.descriptors
        },
        meal_plan_horizon_weeks: to_u32(hh.meal_plan_horizon_weeks),
    })
}

fn slot_view(s: fa_store::MealSlotRow) -> MealSlotView {
    MealSlotView {
        id: Some(s.id),
        name: s.name,
        time: format_slot_time(s.time),
        visible: s.visible,
        ord: to_u32(s.ord),
    }
}

impl MealsService {
    // ---- settings ----

    pub async fn get_settings(&self, h: HouseholdId) -> Result<MealsSettingsView> {
        let mut tx = self.store.tx_for(h).await?;
        let v = settings_view(&mut tx, h).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Absent fields keep their value. The horizon is written to the household.
    pub async fn update_settings(
        &self,
        h: HouseholdId,
        p: MealsSettingsPatch,
    ) -> Result<MealsSettingsView> {
        if let Some(n) = p.grocery_archive_hours
            && !(1..=720).contains(&n)
        {
            return Err(MealsError::invalid("grocery_archive_hours", "1 to 720"));
        }
        if let Some(t) = p.trigram_threshold
            && !(0.2..=0.95).contains(&t)
        {
            return Err(MealsError::invalid("trigram_threshold", "0.2 to 0.95"));
        }
        if let Some(w) = p.meal_plan_horizon_weeks
            && !(1..=52).contains(&w)
        {
            return Err(MealsError::invalid("meal_plan_horizon_weeks", "1 to 52"));
        }
        let descriptors = p.descriptors.map(|d| {
            let mut out: Vec<String> = Vec::new();
            for w in d {
                let w = w.trim().to_lowercase();
                if !w.is_empty() && !out.contains(&w) {
                    out.push(w);
                }
            }
            out
        });
        if descriptors.as_ref().is_some_and(|d| d.len() > 500) {
            return Err(MealsError::invalid("descriptors", "at most 500 words"));
        }
        let mut tx = self.store.tx_for(h).await?;
        MealsSettingsRepo::update(
            &mut tx,
            h,
            &MealsSettingsPatchRow {
                grocery_archive_hours: p.grocery_archive_hours.map(to_i32),
                ingredient_llm: p.ingredient_llm,
                trigram_threshold: p.trigram_threshold,
                descriptors,
            },
        )
        .await?;
        if let Some(w) = p.meal_plan_horizon_weeks {
            HouseholdRepo::set_meal_plan_horizon(&mut tx, h, to_i32(w)).await?;
        }
        publish(&mut tx, h, "mealplan.updated").await?;
        let v = settings_view(&mut tx, h).await?;
        tx.commit().await?;
        Ok(v)
    }

    // ---- slots ----

    pub async fn list_slots(&self, h: HouseholdId) -> Result<Vec<MealSlotView>> {
        let mut tx = self.store.tx_for(h).await?;
        let rows = MealSlotRepo::list(&mut tx).await?;
        tx.rollback().await?;
        Ok(rows.into_iter().map(slot_view).collect())
    }

    /// Replaces the ordered list. A slot that entries or template cells still use cannot go
    /// (`SlotInUse`).
    pub async fn replace_slots(
        &self,
        h: HouseholdId,
        slots: Vec<MealSlotView>,
    ) -> Result<Vec<MealSlotView>> {
        let mut input = Vec::with_capacity(slots.len());
        for s in &slots {
            let time =
                parse_slot_time(&s.time).map_err(|e| MealsError::invalid("time", e.to_string()))?;
            input.push(SlotInput {
                id: s.id,
                name: name_ok("name", &s.name)?,
                time,
                visible: s.visible,
            });
        }
        let mut tx = self.store.tx_for(h).await?;
        let rows = match MealSlotRepo::replace(&mut tx, h, &input).await? {
            SlotReplace::Done(rows) => rows,
            SlotReplace::InUse(_) => return Err(MealsError::SlotInUse),
        };
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(rows.into_iter().map(slot_view).collect())
    }

    // ---- aisles ----

    pub async fn list_aisles(&self, h: HouseholdId) -> Result<Vec<AisleView>> {
        let mut tx = self.store.tx_for(h).await?;
        let rows = AisleRepo::list(&mut tx).await?;
        tx.rollback().await?;
        Ok(rows.into_iter().map(aisle_view).collect())
    }

    pub async fn replace_aisles(
        &self,
        h: HouseholdId,
        aisles: Vec<AisleView>,
    ) -> Result<Vec<AisleView>> {
        let mut rows = Vec::with_capacity(aisles.len());
        for a in &aisles {
            rows.push((a.id, name_ok("name", &a.name)?));
        }
        unique_names("name", rows.iter().map(|(_, n)| n.as_str()))?;
        let mut tx = self.store.tx_for(h).await?;
        let out = AisleRepo::replace(&mut tx, h, &rows).await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(out.into_iter().map(aisle_view).collect())
    }

    // ---- stores ----

    pub async fn list_stores(&self, h: HouseholdId) -> Result<Vec<StoreView>> {
        let mut tx = self.store.tx_for(h).await?;
        let rows = GroceryStoreRepo::list(&mut tx).await?;
        tx.rollback().await?;
        Ok(rows.into_iter().map(store_view).collect())
    }

    /// Removing a store nulls its items and ingredients; a new store copies the aisle order.
    pub async fn replace_stores(
        &self,
        h: HouseholdId,
        stores: Vec<StoreView>,
    ) -> Result<Vec<StoreView>> {
        let mut rows = Vec::with_capacity(stores.len());
        for s in &stores {
            rows.push((s.id, name_ok("name", &s.name)?));
        }
        unique_names("name", rows.iter().map(|(_, n)| n.as_str()))?;
        let mut tx = self.store.tx_for(h).await?;
        let out = GroceryStoreRepo::replace(&mut tx, h, &rows).await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(out.into_iter().map(store_view).collect())
    }

    /// Every aisle in that store's order, with `hidden` (D-121).
    pub async fn store_aisles(
        &self,
        h: HouseholdId,
        store: GroceryStoreId,
    ) -> Result<Vec<StoreAisleView>> {
        let mut tx = self.store.tx_for(h).await?;
        if GroceryStoreRepo::list(&mut tx)
            .await?
            .iter()
            .all(|s| s.id != store)
        {
            return Err(MealsError::NotFound);
        }
        let rows = GroceryStoreRepo::aisles_for(&mut tx, store).await?;
        tx.rollback().await?;
        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| StoreAisleView {
                aisle_id: r.aisle_id,
                ord: u32::try_from(i).unwrap_or(0),
                hidden: r.hidden,
            })
            .collect())
    }

    /// Replaces the store's order: the list is taken in `ord` order, each aisle once.
    pub async fn set_store_aisles(
        &self,
        h: HouseholdId,
        store: GroceryStoreId,
        aisles: Vec<StoreAisleView>,
    ) -> Result<Vec<StoreAisleView>> {
        let mut sorted = aisles;
        sorted.sort_by_key(|a| a.ord);
        let mut seen = HashSet::new();
        if !sorted.iter().all(|a| seen.insert(a.aisle_id)) {
            return Err(MealsError::invalid("aisle_id", "each aisle only once"));
        }
        let mut tx = self.store.tx_for(h).await?;
        let known: HashSet<AisleId> = AisleRepo::list(&mut tx)
            .await?
            .into_iter()
            .map(|a| a.id)
            .collect();
        if sorted.iter().any(|a| !known.contains(&a.aisle_id)) {
            return Err(MealsError::invalid("aisle_id", "unknown aisle"));
        }
        let pairs: Vec<(AisleId, bool)> = sorted.iter().map(|a| (a.aisle_id, a.hidden)).collect();
        GroceryStoreRepo::set_aisles(&mut tx, store, &pairs).await?;
        let rows = GroceryStoreRepo::aisles_for(&mut tx, store).await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(rows
            .into_iter()
            .enumerate()
            .map(|(i, r)| StoreAisleView {
                aisle_id: r.aisle_id,
                ord: u32::try_from(i).unwrap_or(0),
                hidden: r.hidden,
            })
            .collect())
    }
}

fn aisle_view(a: fa_store::AisleRow) -> AisleView {
    AisleView {
        id: Some(a.id),
        name: a.name,
        ord: to_u32(a.ord),
    }
}

fn store_view(s: fa_store::StoreRow) -> StoreView {
    StoreView {
        id: Some(s.id),
        name: s.name,
        ord: to_u32(s.ord),
    }
}
