//! M5 repos: the living grocery list, undoable meal changes and the per-household meals settings.

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{
    AisleId, DeviceId, GroceryItemId, GrocerySection, GroceryStoreId, HouseholdId, IngredientId,
    MealPlanEntryId, MealsChangeId, MealsChangeKind, MemberId,
};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError};

// ---- grocery ----

pub struct GroceryRepo;

#[derive(Debug, Clone, PartialEq)]
pub struct GroceryItemRow {
    pub id: GroceryItemId,
    pub list_id: Uuid,
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
    pub checked_at: Option<DateTime<Utc>>,
    pub checked_by: Option<MemberId>,
    pub archived_at: Option<DateTime<Utc>>,
    pub version: i32,
    pub created_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct RawItem {
    id: Uuid,
    list_id: Uuid,
    ingredient_id: Option<Uuid>,
    text: String,
    qty: Option<f64>,
    unit: Option<String>,
    qty2: Option<f64>,
    unit2: Option<String>,
    store_id: Option<Uuid>,
    aisle_id: Option<Uuid>,
    section: String,
    manual: bool,
    added_by: Option<Uuid>,
    added_via_device: Option<Uuid>,
    checked_at: Option<DateTime<Utc>>,
    checked_by: Option<Uuid>,
    archived_at: Option<DateTime<Utc>>,
    version: i32,
    created_at: DateTime<Utc>,
}

impl TryFrom<RawItem> for GroceryItemRow {
    type Error = StoreError;
    fn try_from(r: RawItem) -> Result<Self> {
        Ok(GroceryItemRow {
            id: r.id.into(),
            list_id: r.list_id,
            ingredient_id: r.ingredient_id.map(Into::into),
            text: r.text,
            qty: r.qty,
            unit: r.unit,
            qty2: r.qty2,
            unit2: r.unit2,
            store_id: r.store_id.map(Into::into),
            aisle_id: r.aisle_id.map(Into::into),
            section: parse_enum(&r.section)?,
            manual: r.manual,
            added_by: r.added_by.map(Into::into),
            added_via_device: r.added_via_device.map(Into::into),
            checked_at: r.checked_at,
            checked_by: r.checked_by.map(Into::into),
            archived_at: r.archived_at,
            version: r.version,
            created_at: r.created_at,
        })
    }
}

const ITEM_COLS: &str = "id, list_id, ingredient_id, text, qty, unit, qty2, unit2, store_id, aisle_id,
    section, manual, added_by, added_via_device, checked_at, checked_by, archived_at, version, created_at";

#[derive(Debug, Clone, PartialEq)]
pub struct NewGroceryItem {
    /// A client-generated id makes a replayed offline add idempotent (D-118).
    pub id: Option<GroceryItemId>,
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

/// The set fields of an adult edit. `Some(None)` clears a nullable field.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct GroceryItemEdit {
    pub text: Option<String>,
    pub qty: Option<Option<f64>>,
    pub unit: Option<Option<String>>,
    pub store_id: Option<Option<GroceryStoreId>>,
}

/// What one meal entry contributed to one item.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct GrocerySourceRow {
    pub item_id: GroceryItemId,
    pub entry_id: MealPlanEntryId,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub qty2: Option<f64>,
    pub unit2: Option<String>,
    pub generated_from: NaiveDate,
    pub generated_to: NaiveDate,
}

/// A source with the entry's date and slot name, for the "for Tue dinner" hint.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct SourceHintRow {
    pub item_id: GroceryItemId,
    pub entry_id: MealPlanEntryId,
    pub date: NaiveDate,
    pub slot_name: String,
}

impl GroceryRepo {
    /// The household's one list, created on first use.
    pub async fn list_id(conn: &mut PgConnection, household: HouseholdId) -> Result<Uuid> {
        sqlx::query(
            "INSERT INTO grocery_list (id, household_id) VALUES ($1, $2)
             ON CONFLICT (household_id) DO NOTHING",
        )
        .bind(Uuid::now_v7())
        .bind(household.as_uuid())
        .execute(&mut *conn)
        .await?;
        Ok(
            sqlx::query_scalar("SELECT id FROM grocery_list WHERE household_id = $1")
                .bind(household.as_uuid())
                .fetch_one(conn)
                .await?,
        )
    }

    /// Unarchived items (checked ones included), oldest first.
    pub async fn items_open(conn: &mut PgConnection, list: Uuid) -> Result<Vec<GroceryItemRow>> {
        let sql = format!(
            "SELECT {ITEM_COLS} FROM grocery_item WHERE list_id = $1 AND archived_at IS NULL
             ORDER BY created_at, id"
        );
        let rows = sqlx::query_as::<_, RawItem>(&sql)
            .bind(list)
            .fetch_all(conn)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get_item(
        conn: &mut PgConnection,
        id: GroceryItemId,
    ) -> Result<Option<GroceryItemRow>> {
        let sql = format!("SELECT {ITEM_COLS} FROM grocery_item WHERE id = $1");
        let row = sqlx::query_as::<_, RawItem>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Inserts an item. With a client id that already exists nothing is written: the existing
    /// row comes back with `false`. Otherwise `true`.
    pub async fn insert_item(
        conn: &mut PgConnection,
        household: HouseholdId,
        list: Uuid,
        n: &NewGroceryItem,
    ) -> Result<(GroceryItemRow, bool)> {
        let id = n.id.unwrap_or_default();
        let sql = format!(
            "INSERT INTO grocery_item (id, household_id, list_id, ingredient_id, text, qty, unit, qty2,
                                       unit2, store_id, aisle_id, section, manual, added_by,
                                       added_via_device)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15)
             ON CONFLICT (id) DO NOTHING RETURNING {ITEM_COLS}"
        );
        let inserted = sqlx::query_as::<_, RawItem>(&sql)
            .bind(id.as_uuid())
            .bind(household.as_uuid())
            .bind(list)
            .bind(n.ingredient_id.map(|i| i.as_uuid()))
            .bind(&n.text)
            .bind(n.qty)
            .bind(&n.unit)
            .bind(n.qty2)
            .bind(&n.unit2)
            .bind(n.store_id.map(|i| i.as_uuid()))
            .bind(n.aisle_id.map(|i| i.as_uuid()))
            .bind(enum_str(&n.section))
            .bind(n.manual)
            .bind(n.added_by.map(|i| i.as_uuid()))
            .bind(n.added_via_device.map(|i| i.as_uuid()))
            .fetch_optional(&mut *conn)
            .await?;
        match inserted {
            Some(r) => Ok((r.try_into()?, true)),
            None => {
                let existing = Self::get_item(conn, id)
                    .await?
                    .ok_or(StoreError::NotFound)?;
                Ok((existing, false))
            }
        }
    }

    /// Checks an item off. Idempotent and independent of the stored version (check-offs win):
    /// an already checked item is returned unchanged.
    pub async fn check(
        conn: &mut PgConnection,
        id: GroceryItemId,
        by: Option<MemberId>,
    ) -> Result<GroceryItemRow> {
        sqlx::query(
            "UPDATE grocery_item SET checked_at = now(), checked_by = $2, version = version + 1
             WHERE id = $1 AND checked_at IS NULL",
        )
        .bind(id.as_uuid())
        .bind(by.map(|m| m.as_uuid()))
        .execute(&mut *conn)
        .await?;
        Self::get_item(conn, id).await?.ok_or(StoreError::NotFound)
    }

    /// The reverse of [`Self::check`]; also idempotent.
    pub async fn uncheck(conn: &mut PgConnection, id: GroceryItemId) -> Result<GroceryItemRow> {
        sqlx::query(
            "UPDATE grocery_item SET checked_at = NULL, checked_by = NULL, version = version + 1
             WHERE id = $1 AND checked_at IS NOT NULL",
        )
        .bind(id.as_uuid())
        .execute(&mut *conn)
        .await?;
        Self::get_item(conn, id).await?.ok_or(StoreError::NotFound)
    }

    /// Moves an item between the list and the staples check (the "low" route); unchecks it.
    pub async fn set_section(
        conn: &mut PgConnection,
        id: GroceryItemId,
        section: GrocerySection,
    ) -> Result<GroceryItemRow> {
        sqlx::query(
            "UPDATE grocery_item SET section = $2, checked_at = NULL, checked_by = NULL,
                    version = version + 1
             WHERE id = $1 AND section <> $2",
        )
        .bind(id.as_uuid())
        .bind(enum_str(&section))
        .execute(&mut *conn)
        .await?;
        Self::get_item(conn, id).await?.ok_or(StoreError::NotFound)
    }

    /// An adult edit. Returns the new version, or `Conflict` if stale.
    pub async fn update_item(
        conn: &mut PgConnection,
        id: GroceryItemId,
        p: &GroceryItemEdit,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE grocery_item SET
                    text = COALESCE($2, text),
                    qty = CASE WHEN $3 THEN $4 ELSE qty END,
                    unit = CASE WHEN $5 THEN $6 ELSE unit END,
                    store_id = CASE WHEN $7 THEN $8 ELSE store_id END,
                    version = version + 1
             WHERE id = $1 AND version = $9
             RETURNING version",
        )
        .bind(id.as_uuid())
        .bind(&p.text)
        .bind(p.qty.is_some())
        .bind(p.qty.flatten())
        .bind(p.unit.is_some())
        .bind(p.unit.clone().flatten())
        .bind(p.store_id.is_some())
        .bind(p.store_id.flatten().map(|s| s.as_uuid()))
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "grocery_item", id.as_uuid()).await),
        }
    }

    /// Rewrites the generated quantities (merge, reconcile, undo). No version check.
    pub async fn set_quantities(
        conn: &mut PgConnection,
        id: GroceryItemId,
        qty: Option<f64>,
        unit: Option<&str>,
        qty2: Option<f64>,
        unit2: Option<&str>,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE grocery_item SET qty = $2, unit = $3, qty2 = $4, unit2 = $5, version = version + 1
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(qty)
        .bind(unit)
        .bind(qty2)
        .bind(unit2)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    pub async fn delete_item(conn: &mut PgConnection, id: GroceryItemId) -> Result<()> {
        let n = sqlx::query("DELETE FROM grocery_item WHERE id = $1")
            .bind(id.as_uuid())
            .execute(conn)
            .await?
            .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// The sources that point at any of these entries.
    pub async fn sources_for_entries(
        conn: &mut PgConnection,
        entries: &[MealPlanEntryId],
    ) -> Result<Vec<GrocerySourceRow>> {
        let ids: Vec<Uuid> = entries.iter().map(|e| e.as_uuid()).collect();
        Ok(sqlx::query_as::<_, GrocerySourceRow>(
            "SELECT item_id, entry_id, qty, unit, qty2, unit2, generated_from, generated_to
             FROM grocery_item_source WHERE entry_id = ANY ($1) ORDER BY item_id, entry_id",
        )
        .bind(&ids)
        .fetch_all(conn)
        .await?)
    }

    /// The sources of these items, with the entry's date and slot name.
    pub async fn hints_for_items(
        conn: &mut PgConnection,
        items: &[GroceryItemId],
    ) -> Result<Vec<SourceHintRow>> {
        let ids: Vec<Uuid> = items.iter().map(|e| e.as_uuid()).collect();
        Ok(sqlx::query_as::<_, SourceHintRow>(
            "SELECT s.item_id, s.entry_id, e.date, m.name AS slot_name
             FROM grocery_item_source s
             JOIN meal_plan_entry e ON e.id = s.entry_id
             JOIN meal_slot m ON m.id = e.slot_id
             WHERE s.item_id = ANY ($1) ORDER BY e.date, m.ord, e.ord, s.entry_id",
        )
        .bind(&ids)
        .fetch_all(conn)
        .await?)
    }

    pub async fn upsert_source(
        conn: &mut PgConnection,
        household: HouseholdId,
        s: &GrocerySourceRow,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO grocery_item_source (item_id, entry_id, household_id, qty, unit, qty2, unit2,
                                              generated_from, generated_to)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)
             ON CONFLICT (item_id, entry_id) DO UPDATE
                SET qty = EXCLUDED.qty, unit = EXCLUDED.unit, qty2 = EXCLUDED.qty2,
                    unit2 = EXCLUDED.unit2, generated_from = EXCLUDED.generated_from,
                    generated_to = EXCLUDED.generated_to",
        )
        .bind(s.item_id.as_uuid())
        .bind(s.entry_id.as_uuid())
        .bind(household.as_uuid())
        .bind(s.qty)
        .bind(&s.unit)
        .bind(s.qty2)
        .bind(&s.unit2)
        .bind(s.generated_from)
        .bind(s.generated_to)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn delete_source(
        conn: &mut PgConnection,
        item: GroceryItemId,
        entry: MealPlanEntryId,
    ) -> Result<()> {
        sqlx::query("DELETE FROM grocery_item_source WHERE item_id = $1 AND entry_id = $2")
            .bind(item.as_uuid())
            .bind(entry.as_uuid())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Archives items checked before `cutoff`. Runs inside one household's transaction (the
    /// archive hours are per household). Returns how many were archived.
    pub async fn archive_checked_before(
        conn: &mut PgConnection,
        cutoff: DateTime<Utc>,
    ) -> Result<u64> {
        Ok(sqlx::query(
            "UPDATE grocery_item SET archived_at = now(), version = version + 1
             WHERE archived_at IS NULL AND checked_at IS NOT NULL AND checked_at < $1",
        )
        .bind(cutoff)
        .execute(conn)
        .await?
        .rows_affected())
    }
}

// ---- undoable changes ----

pub struct MealsChangeRepo;

#[derive(Debug, Clone, PartialEq)]
pub struct MealsChangeRow {
    pub id: MealsChangeId,
    pub household_id: HouseholdId,
    pub kind: MealsChangeKind,
    pub actor_member: Option<MemberId>,
    pub actor_device: Option<DeviceId>,
    pub inverse: Value,
    pub expires_at: DateTime<Utc>,
    pub undone_at: Option<DateTime<Utc>>,
}

#[derive(FromRow)]
struct RawChange {
    id: Uuid,
    household_id: Uuid,
    kind: String,
    actor_member: Option<Uuid>,
    actor_device: Option<Uuid>,
    inverse: Value,
    expires_at: DateTime<Utc>,
    undone_at: Option<DateTime<Utc>>,
}

impl TryFrom<RawChange> for MealsChangeRow {
    type Error = StoreError;
    fn try_from(r: RawChange) -> Result<Self> {
        Ok(MealsChangeRow {
            id: r.id.into(),
            household_id: r.household_id.into(),
            kind: parse_enum(&r.kind)?,
            actor_member: r.actor_member.map(Into::into),
            actor_device: r.actor_device.map(Into::into),
            inverse: r.inverse,
            expires_at: r.expires_at,
            undone_at: r.undone_at,
        })
    }
}

const CHANGE_COLS: &str =
    "id, household_id, kind, actor_member, actor_device, inverse, expires_at, undone_at";

impl MealsChangeRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        household: HouseholdId,
        kind: MealsChangeKind,
        actor_member: Option<MemberId>,
        actor_device: Option<DeviceId>,
        inverse: &Value,
        expires_at: DateTime<Utc>,
    ) -> Result<MealsChangeRow> {
        let sql = format!(
            "INSERT INTO meals_change (id, household_id, kind, actor_member, actor_device, inverse, expires_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7) RETURNING {CHANGE_COLS}"
        );
        let row = sqlx::query_as::<_, RawChange>(&sql)
            .bind(MealsChangeId::new().as_uuid())
            .bind(household.as_uuid())
            .bind(enum_str(&kind))
            .bind(actor_member.map(|m| m.as_uuid()))
            .bind(actor_device.map(|m| m.as_uuid()))
            .bind(inverse)
            .bind(expires_at)
            .fetch_one(conn)
            .await?;
        row.try_into()
    }

    pub async fn get(conn: &mut PgConnection, id: MealsChangeId) -> Result<Option<MealsChangeRow>> {
        let sql = format!("SELECT {CHANGE_COLS} FROM meals_change WHERE id = $1");
        let row = sqlx::query_as::<_, RawChange>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Marks the change undone. Returns false when it already was (or does not exist).
    pub async fn mark_undone(conn: &mut PgConnection, id: MealsChangeId) -> Result<bool> {
        Ok(sqlx::query(
            "UPDATE meals_change SET undone_at = now() WHERE id = $1 AND undone_at IS NULL",
        )
        .bind(id.as_uuid())
        .execute(conn)
        .await?
        .rows_affected()
            > 0)
    }

    /// Deletes changes whose `expires_at` is before `cutoff`. Returns how many.
    pub async fn prune_before(conn: &mut PgConnection, cutoff: DateTime<Utc>) -> Result<u64> {
        Ok(
            sqlx::query("DELETE FROM meals_change WHERE expires_at < $1")
                .bind(cutoff)
                .execute(conn)
                .await?
                .rows_affected(),
        )
    }
}

// ---- settings ----

pub struct MealsSettingsRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MealsSettingsRow {
    pub grocery_archive_hours: i32,
    pub ingredient_llm: bool,
    pub trigram_threshold: f32,
    /// Empty means the seed descriptor list.
    pub descriptors: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Default)]
pub struct MealsSettingsPatchRow {
    pub grocery_archive_hours: Option<i32>,
    pub ingredient_llm: Option<bool>,
    pub trigram_threshold: Option<f32>,
    pub descriptors: Option<Vec<String>>,
}

const SETTINGS_COLS: &str = "grocery_archive_hours, ingredient_llm, trigram_threshold, descriptors";

impl MealsSettingsRepo {
    /// The household's row, created with the defaults on first use.
    pub async fn get_or_default(
        conn: &mut PgConnection,
        household: HouseholdId,
    ) -> Result<MealsSettingsRow> {
        sqlx::query("INSERT INTO meals_settings (household_id) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(household.as_uuid())
            .execute(&mut *conn)
            .await?;
        let sql = format!("SELECT {SETTINGS_COLS} FROM meals_settings WHERE household_id = $1");
        Ok(sqlx::query_as::<_, MealsSettingsRow>(&sql)
            .bind(household.as_uuid())
            .fetch_one(conn)
            .await?)
    }

    /// Applies the set fields (absent keeps) and returns the row.
    pub async fn update(
        conn: &mut PgConnection,
        household: HouseholdId,
        p: &MealsSettingsPatchRow,
    ) -> Result<MealsSettingsRow> {
        Self::get_or_default(conn, household).await?;
        let sql = format!(
            "UPDATE meals_settings SET
                    grocery_archive_hours = COALESCE($2, grocery_archive_hours),
                    ingredient_llm = COALESCE($3, ingredient_llm),
                    trigram_threshold = COALESCE($4, trigram_threshold),
                    descriptors = COALESCE($5, descriptors)
             WHERE household_id = $1 RETURNING {SETTINGS_COLS}"
        );
        Ok(sqlx::query_as::<_, MealsSettingsRow>(&sql)
            .bind(household.as_uuid())
            .bind(p.grocery_archive_hours)
            .bind(p.ingredient_llm)
            .bind(p.trigram_threshold)
            .bind(&p.descriptors)
            .fetch_one(conn)
            .await?)
    }
}
