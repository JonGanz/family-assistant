//! Undoable multi-row changes (`meals_change`): the inverse payloads, recording, and the
//! "pending" records that keep a deleted entry's grocery contributions until the list is
//! reconciled.
//!
//! `grocery_item_source.entry_id` cascades on delete, so when a plan entry is deleted its
//! sources vanish with it. The plan routes therefore park them in a `pending` change first, and
//! `reconcile` reads them back for entries that no longer exist.

use chrono::{DateTime, Duration, NaiveDate, Utc};
use fa_domain::*;
use fa_store::{
    GroceryItemRow, GrocerySourceRow, MealEntryRow, MealPlanRepo, MealsChangeRepo, MealsChangeRow,
};
use serde::{Deserialize, Serialize};
use sqlx::PgConnection;
use uuid::Uuid;

use crate::error::{MealsError, Result};
use crate::service::{ActorRef, UNDO_MINUTES};

/// How long a parked contribution waits for the "Update the shopping list?" answer.
const PENDING_HOURS: i64 = 6;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EntrySnap {
    pub id: MealPlanEntryId,
    pub date: NaiveDate,
    pub slot_id: MealSlotId,
    pub ord: i32,
    pub recipe_id: Option<RecipeId>,
    pub free_text: Option<String>,
    pub title: String,
    pub servings: Option<i32>,
    pub created_by: Option<MemberId>,
    pub version: i32,
}

impl From<&MealEntryRow> for EntrySnap {
    fn from(r: &MealEntryRow) -> Self {
        Self {
            id: r.id,
            date: r.date,
            slot_id: r.slot_id,
            ord: r.ord,
            recipe_id: r.recipe_id,
            free_text: r.free_text.clone(),
            title: r.title.clone(),
            servings: r.servings,
            created_by: r.created_by,
            version: r.version,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SrcSnap {
    pub item_id: GroceryItemId,
    pub entry_id: MealPlanEntryId,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub qty2: Option<f64>,
    pub unit2: Option<String>,
    pub generated_from: NaiveDate,
    pub generated_to: NaiveDate,
}

impl From<&GrocerySourceRow> for SrcSnap {
    fn from(s: &GrocerySourceRow) -> Self {
        Self {
            item_id: s.item_id,
            entry_id: s.entry_id,
            qty: s.qty,
            unit: s.unit.clone(),
            qty2: s.qty2,
            unit2: s.unit2.clone(),
            generated_from: s.generated_from,
            generated_to: s.generated_to,
        }
    }
}

impl From<SrcSnap> for GrocerySourceRow {
    fn from(s: SrcSnap) -> Self {
        Self {
            item_id: s.item_id,
            entry_id: s.entry_id,
            qty: s.qty,
            unit: s.unit,
            qty2: s.qty2,
            unit2: s.unit2,
            generated_from: s.generated_from,
            generated_to: s.generated_to,
        }
    }
}

/// An item as it was, enough to put it back.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemSnap {
    pub id: GroceryItemId,
    pub ingredient_id: Option<IngredientId>,
    pub text: String,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub qty2: Option<f64>,
    pub unit2: Option<String>,
    pub store_id: Option<GroceryStoreId>,
    pub aisle_id: Option<AisleId>,
    pub section: GrocerySection,
    pub manual: bool,
    pub added_by: Option<MemberId>,
    pub added_via_device: Option<DeviceId>,
}

impl From<&GroceryItemRow> for ItemSnap {
    fn from(i: &GroceryItemRow) -> Self {
        Self {
            id: i.id,
            ingredient_id: i.ingredient_id,
            text: i.text.clone(),
            qty: i.qty,
            unit: i.unit.clone(),
            qty2: i.qty2,
            unit2: i.unit2.clone(),
            store_id: i.store_id,
            aisle_id: i.aisle_id,
            section: i.section,
            manual: i.manual,
            added_by: i.added_by,
            added_via_device: i.added_via_device,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ItemUndo {
    pub item: ItemSnap,
    /// The item was deleted by the change.
    pub removed: bool,
    /// The sources of the touched entries as they were.
    pub sources: Vec<SrcSnap>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Inverse {
    Swap {
        a: MealPlanEntryId,
        b: MealPlanEntryId,
    },
    Move {
        entry: MealPlanEntryId,
        date: NaiveDate,
        slot_id: MealSlotId,
        ord: i32,
    },
    TemplateApply {
        deleted: Vec<EntrySnap>,
        created: Vec<MealPlanEntryId>,
    },
    Generate {
        created_items: Vec<GroceryItemId>,
        restore_items: Vec<ItemSnap>,
        added_sources: Vec<(GroceryItemId, MealPlanEntryId)>,
    },
    Reconcile {
        items: Vec<ItemUndo>,
    },
    /// Not undoable: a deleted entry's contributions, waiting for `reconcile`.
    Pending {
        entry_id: MealPlanEntryId,
        sources: Vec<SrcSnap>,
    },
}

impl Inverse {
    pub fn is_pending(&self) -> bool {
        matches!(self, Inverse::Pending { .. })
    }
}

pub(crate) fn to_json(i: &Inverse) -> Result<serde_json::Value> {
    serde_json::to_value(i)
        .map_err(|e| MealsError::Store(fa_store::StoreError::Corrupt(e.to_string())))
}

pub(crate) fn from_json(v: &serde_json::Value) -> Result<Inverse> {
    serde_json::from_value(v.clone())
        .map_err(|e| MealsError::Store(fa_store::StoreError::Corrupt(e.to_string())))
}

pub(crate) async fn record(
    conn: &mut PgConnection,
    h: HouseholdId,
    actor: ActorRef,
    kind: MealsChangeKind,
    inverse: &Inverse,
    now: DateTime<Utc>,
) -> Result<MealsChangeRow> {
    Ok(MealsChangeRepo::insert(
        conn,
        h,
        kind,
        actor.member,
        actor.device,
        &to_json(inverse)?,
        now + Duration::minutes(UNDO_MINUTES),
    )
    .await?)
}

/// Parks the contributions of entries that are about to be deleted.
pub(crate) async fn park(
    conn: &mut PgConnection,
    h: HouseholdId,
    sources: &[GrocerySourceRow],
    now: DateTime<Utc>,
) -> Result<()> {
    let mut entries: Vec<MealPlanEntryId> = Vec::new();
    for s in sources {
        if !entries.contains(&s.entry_id) {
            entries.push(s.entry_id);
        }
    }
    for e in entries {
        let snaps: Vec<SrcSnap> = sources
            .iter()
            .filter(|s| s.entry_id == e)
            .map(SrcSnap::from)
            .collect();
        MealsChangeRepo::insert(
            conn,
            h,
            MealsChangeKind::Reconcile,
            None,
            None,
            &to_json(&Inverse::Pending {
                entry_id: e,
                sources: snaps,
            })?,
            now + Duration::hours(PENDING_HOURS),
        )
        .await?;
    }
    Ok(())
}

/// Parked contributions of these (now deleted) entries: `(change id, entry, sources)`.
pub(crate) async fn parked_for(
    conn: &mut PgConnection,
    entries: &[MealPlanEntryId],
    now: DateTime<Utc>,
) -> Result<Vec<(MealsChangeId, MealPlanEntryId, Vec<SrcSnap>)>> {
    let ids: Vec<String> = entries.iter().map(ToString::to_string).collect();
    let rows: Vec<(Uuid, serde_json::Value)> = sqlx::query_as(
        "SELECT id, inverse FROM meals_change
         WHERE kind = 'reconcile' AND undone_at IS NULL AND expires_at > $2
           AND inverse->>'type' = 'pending' AND inverse->>'entry_id' = ANY ($1)
         ORDER BY created_at, id",
    )
    .bind(&ids)
    .bind(now)
    .fetch_all(conn)
    .await?;
    let mut out = Vec::new();
    for (id, inv) in rows {
        if let Inverse::Pending { entry_id, sources } = from_json(&inv)? {
            out.push((MealsChangeId::from(id), entry_id, sources));
        }
    }
    Ok(out)
}

/// Re-inserts an entry from its snapshot. A recipe that has since been deleted becomes free text.
pub(crate) async fn restore_entry(
    conn: &mut PgConnection,
    h: HouseholdId,
    s: &EntrySnap,
) -> Result<()> {
    let recipe_ok = match s.recipe_id {
        Some(r) => fa_store::RecipeRepo::get(conn, r).await?.is_some(),
        None => false,
    };
    let row = MealEntryRow {
        id: s.id,
        date: s.date,
        slot_id: s.slot_id,
        ord: s.ord,
        recipe_id: s.recipe_id.filter(|_| recipe_ok),
        title: s.title.clone(),
        free_text: if recipe_ok {
            None
        } else {
            Some(s.free_text.clone().unwrap_or_else(|| s.title.clone()))
        },
        servings: s.servings,
        created_by: s.created_by,
        version: s.version,
    };
    MealPlanRepo::restore(conn, h, &row).await?;
    Ok(())
}
