//! M5 repos: aisles, stores, meal slots, the meal plan and templates.

use chrono::{NaiveDate, NaiveTime};
use fa_domain::{
    AisleId, GroceryStoreId, HouseholdId, MealPlanEntryId, MealSlotId, MealTemplateId, MemberId,
    RecipeId,
};
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::conflict_or_missing;
use crate::{Result, StoreError};

fn ord_i32(i: usize) -> Result<i32> {
    i32::try_from(i).map_err(|_| StoreError::Corrupt("too many rows".into()))
}

// ---- aisles ----

pub struct AisleRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct AisleRow {
    pub id: AisleId,
    pub name: String,
    pub ord: i32,
}

impl AisleRepo {
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<AisleRow>> {
        Ok(sqlx::query_as::<_, AisleRow>(
            "SELECT id, name, ord FROM aisle ORDER BY ord, lower(name), id",
        )
        .fetch_all(conn)
        .await?)
    }

    pub async fn count(conn: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM aisle")
            .fetch_one(conn)
            .await?)
    }

    /// Replaces the ordered list. A row with an id keeps it (rename and reorder); a row without
    /// one is inserted; aisles not listed are deleted (items and ingredients fall back to
    /// "Other"). Returns the rows in order.
    pub async fn replace(
        conn: &mut PgConnection,
        household: HouseholdId,
        rows: &[(Option<AisleId>, String)],
    ) -> Result<Vec<AisleRow>> {
        let keep: Vec<Uuid> = rows
            .iter()
            .filter_map(|(i, _)| i.map(|i| i.as_uuid()))
            .collect();
        sqlx::query("DELETE FROM aisle WHERE NOT (id = ANY ($1))")
            .bind(&keep)
            .execute(&mut *conn)
            .await?;
        // Free the names first so a swap of two names never trips the unique index.
        sqlx::query("UPDATE aisle SET name = id::text WHERE id = ANY ($1)")
            .bind(&keep)
            .execute(&mut *conn)
            .await?;
        for (ord, (id, name)) in rows.iter().enumerate() {
            let ord = ord_i32(ord)?;
            match id {
                Some(id) => {
                    let n = sqlx::query("UPDATE aisle SET name = $2, ord = $3 WHERE id = $1")
                        .bind(id.as_uuid())
                        .bind(name)
                        .bind(ord)
                        .execute(&mut *conn)
                        .await?
                        .rows_affected();
                    if n == 0 {
                        return Err(StoreError::NotFound);
                    }
                }
                None => {
                    sqlx::query(
                        "INSERT INTO aisle (id, household_id, name, ord) VALUES ($1, $2, $3, $4)",
                    )
                    .bind(AisleId::new().as_uuid())
                    .bind(household.as_uuid())
                    .bind(name)
                    .bind(ord)
                    .execute(&mut *conn)
                    .await?;
                }
            }
        }
        Self::list(conn).await
    }
}

// ---- stores ----

pub struct GroceryStoreRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StoreRow {
    pub id: GroceryStoreId,
    pub name: String,
    pub ord: i32,
}

/// One aisle in a store's own order (D-121).
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct StoreAisleRow {
    pub aisle_id: AisleId,
    pub ord: i32,
    pub hidden: bool,
}

impl GroceryStoreRepo {
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<StoreRow>> {
        Ok(sqlx::query_as::<_, StoreRow>(
            "SELECT id, name, ord FROM grocery_store ORDER BY ord, lower(name), id",
        )
        .fetch_all(conn)
        .await?)
    }

    /// Replaces the ordered list like [`AisleRepo::replace`]. A new store gets one
    /// `grocery_store_aisle` row per aisle, copied from `aisle.ord`. Removing a store nulls its
    /// items and ingredients (SET NULL).
    pub async fn replace(
        conn: &mut PgConnection,
        household: HouseholdId,
        rows: &[(Option<GroceryStoreId>, String)],
    ) -> Result<Vec<StoreRow>> {
        let keep: Vec<Uuid> = rows
            .iter()
            .filter_map(|(i, _)| i.map(|i| i.as_uuid()))
            .collect();
        sqlx::query("DELETE FROM grocery_store WHERE NOT (id = ANY ($1))")
            .bind(&keep)
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE grocery_store SET name = id::text WHERE id = ANY ($1)")
            .bind(&keep)
            .execute(&mut *conn)
            .await?;
        for (ord, (id, name)) in rows.iter().enumerate() {
            let ord = ord_i32(ord)?;
            match id {
                Some(id) => {
                    let n =
                        sqlx::query("UPDATE grocery_store SET name = $2, ord = $3 WHERE id = $1")
                            .bind(id.as_uuid())
                            .bind(name)
                            .bind(ord)
                            .execute(&mut *conn)
                            .await?
                            .rows_affected();
                    if n == 0 {
                        return Err(StoreError::NotFound);
                    }
                }
                None => {
                    let id = GroceryStoreId::new();
                    sqlx::query(
                        "INSERT INTO grocery_store (id, household_id, name, ord)
                         VALUES ($1, $2, $3, $4)",
                    )
                    .bind(id.as_uuid())
                    .bind(household.as_uuid())
                    .bind(name)
                    .bind(ord)
                    .execute(&mut *conn)
                    .await?;
                    sqlx::query(
                        "INSERT INTO grocery_store_aisle (store_id, aisle_id, household_id, ord)
                         SELECT $1, id, household_id, ord FROM aisle",
                    )
                    .bind(id.as_uuid())
                    .execute(&mut *conn)
                    .await?;
                }
            }
        }
        Self::list(conn).await
    }

    /// Every aisle of the household in this store's order. An aisle with no
    /// `grocery_store_aisle` row falls back to `aisle.ord` and is not hidden.
    pub async fn aisles_for(
        conn: &mut PgConnection,
        store: GroceryStoreId,
    ) -> Result<Vec<StoreAisleRow>> {
        Ok(sqlx::query_as::<_, StoreAisleRow>(
            "SELECT a.id AS aisle_id, COALESCE(s.ord, a.ord) AS ord, COALESCE(s.hidden, false) AS hidden
             FROM aisle a
             LEFT JOIN grocery_store_aisle s ON s.aisle_id = a.id AND s.store_id = $1
             ORDER BY 2, a.ord, lower(a.name), a.id",
        )
        .bind(store.as_uuid())
        .fetch_all(conn)
        .await?)
    }

    /// Replaces the store's aisle order: position `i` of `aisles` becomes `ord = i`. Pass every
    /// aisle; any left out falls back to the household order again.
    pub async fn set_aisles(
        conn: &mut PgConnection,
        store: GroceryStoreId,
        aisles: &[(AisleId, bool)],
    ) -> Result<()> {
        let exists: bool =
            sqlx::query_scalar("SELECT EXISTS (SELECT 1 FROM grocery_store WHERE id = $1)")
                .bind(store.as_uuid())
                .fetch_one(&mut *conn)
                .await?;
        if !exists {
            return Err(StoreError::NotFound);
        }
        sqlx::query("DELETE FROM grocery_store_aisle WHERE store_id = $1")
            .bind(store.as_uuid())
            .execute(&mut *conn)
            .await?;
        for (ord, (aisle, hidden)) in aisles.iter().enumerate() {
            sqlx::query(
                "INSERT INTO grocery_store_aisle (store_id, aisle_id, household_id, ord, hidden)
                 SELECT $1, a.id, a.household_id, $3, $4 FROM aisle a WHERE a.id = $2",
            )
            .bind(store.as_uuid())
            .bind(aisle.as_uuid())
            .bind(ord_i32(ord)?)
            .bind(hidden)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
}

// ---- meal slots ----

pub struct MealSlotRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MealSlotRow {
    pub id: MealSlotId,
    pub name: String,
    pub ord: i32,
    pub time: NaiveTime,
    pub visible: bool,
}

/// A slot as `PUT /meals/slots` sends it.
#[derive(Debug, Clone, PartialEq)]
pub struct SlotInput {
    pub id: Option<MealSlotId>,
    pub name: String,
    pub time: NaiveTime,
    pub visible: bool,
}

/// The outcome of [`MealSlotRepo::replace`].
#[derive(Debug, Clone, PartialEq)]
pub enum SlotReplace {
    Done(Vec<MealSlotRow>),
    /// A slot to remove still has plan entries or template cells (nothing was changed).
    InUse(MealSlotId),
}

impl MealSlotRepo {
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<MealSlotRow>> {
        Ok(sqlx::query_as::<_, MealSlotRow>(
            "SELECT id, name, ord, time, visible FROM meal_slot ORDER BY ord, time, id",
        )
        .fetch_all(conn)
        .await?)
    }

    pub async fn get(conn: &mut PgConnection, id: MealSlotId) -> Result<Option<MealSlotRow>> {
        Ok(sqlx::query_as::<_, MealSlotRow>(
            "SELECT id, name, ord, time, visible FROM meal_slot WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?)
    }

    pub async fn count(conn: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM meal_slot")
            .fetch_one(conn)
            .await?)
    }

    pub async fn insert(
        conn: &mut PgConnection,
        household: HouseholdId,
        name: &str,
        ord: i32,
        time: NaiveTime,
        visible: bool,
    ) -> Result<MealSlotRow> {
        Ok(sqlx::query_as::<_, MealSlotRow>(
            "INSERT INTO meal_slot (id, household_id, name, ord, time, visible)
             VALUES ($1, $2, $3, $4, $5, $6) RETURNING id, name, ord, time, visible",
        )
        .bind(MealSlotId::new().as_uuid())
        .bind(household.as_uuid())
        .bind(name)
        .bind(ord)
        .bind(time)
        .bind(visible)
        .fetch_one(conn)
        .await?)
    }

    /// Whether any plan entry or template cell uses the slot.
    pub async fn in_use(conn: &mut PgConnection, id: MealSlotId) -> Result<bool> {
        Ok(sqlx::query_scalar(
            "SELECT EXISTS (SELECT 1 FROM meal_plan_entry WHERE slot_id = $1)
                 OR EXISTS (SELECT 1 FROM meal_template_cell WHERE slot_id = $1)",
        )
        .bind(id.as_uuid())
        .fetch_one(conn)
        .await?)
    }

    /// Replaces the ordered slot list: ids are kept, new rows inserted, the rest removed. A slot
    /// that is still used is never removed (`SlotReplace::InUse`, nothing changed).
    pub async fn replace(
        conn: &mut PgConnection,
        household: HouseholdId,
        slots: &[SlotInput],
    ) -> Result<SlotReplace> {
        let keep: Vec<Uuid> = slots
            .iter()
            .filter_map(|s| s.id.map(|i| i.as_uuid()))
            .collect();
        let dropped: Vec<Uuid> =
            sqlx::query_scalar("SELECT id FROM meal_slot WHERE NOT (id = ANY ($1))")
                .bind(&keep)
                .fetch_all(&mut *conn)
                .await?;
        for id in dropped {
            let id = MealSlotId::from(id);
            if Self::in_use(conn, id).await? {
                return Ok(SlotReplace::InUse(id));
            }
        }
        sqlx::query("DELETE FROM meal_slot WHERE NOT (id = ANY ($1))")
            .bind(&keep)
            .execute(&mut *conn)
            .await?;
        for (ord, s) in slots.iter().enumerate() {
            let ord = ord_i32(ord)?;
            match s.id {
                Some(id) => {
                    let n = sqlx::query(
                        "UPDATE meal_slot SET name = $2, ord = $3, time = $4, visible = $5 WHERE id = $1",
                    )
                    .bind(id.as_uuid())
                    .bind(&s.name)
                    .bind(ord)
                    .bind(s.time)
                    .bind(s.visible)
                    .execute(&mut *conn)
                    .await?
                    .rows_affected();
                    if n == 0 {
                        return Err(StoreError::NotFound);
                    }
                }
                None => {
                    Self::insert(conn, household, &s.name, ord, s.time, s.visible).await?;
                }
            }
        }
        Ok(SlotReplace::Done(Self::list(conn).await?))
    }
}

// ---- meal plan ----

pub struct MealPlanRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MealEntryRow {
    pub id: MealPlanEntryId,
    pub date: NaiveDate,
    pub slot_id: MealSlotId,
    pub ord: i32,
    pub recipe_id: Option<RecipeId>,
    /// The recipe's title, or the free text.
    pub title: String,
    pub free_text: Option<String>,
    pub servings: Option<i32>,
    pub created_by: Option<MemberId>,
    pub version: i32,
}

const ENTRY_SELECT: &str = "SELECT e.id, e.date, e.slot_id, e.ord, e.recipe_id,
        COALESCE(r.title, e.free_text, '') AS title, e.free_text, e.servings, e.created_by, e.version
     FROM meal_plan_entry e LEFT JOIN recipe r ON r.id = e.recipe_id";

#[derive(Debug, Clone, PartialEq)]
pub enum EntryContent {
    Recipe(RecipeId),
    FreeText(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewEntry {
    pub date: NaiveDate,
    pub slot_id: MealSlotId,
    pub content: EntryContent,
    pub servings: Option<i32>,
    pub created_by: Option<MemberId>,
}

/// The set fields of an update. Moving to another date or slot appends to the end of it.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct EntryPatch {
    pub date: Option<NaiveDate>,
    pub slot_id: Option<MealSlotId>,
    pub content: Option<EntryContent>,
    /// Absent = keep, `Some(None)` = clear.
    pub servings: Option<Option<i32>>,
}

/// One planned meal for the dashboard (visible slots only).
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct DayMealRow {
    pub slot_name: String,
    pub slot_time: NaiveTime,
    pub title: String,
    pub recipe_id: Option<RecipeId>,
}

fn split(c: &EntryContent) -> (Option<Uuid>, Option<&str>) {
    match c {
        EntryContent::Recipe(r) => (Some(r.as_uuid()), None),
        EntryContent::FreeText(t) => (None, Some(t.as_str())),
    }
}

impl MealPlanRepo {
    /// Entries from `from` to `to`, both inclusive, ordered by date, slot order, `ord`.
    pub async fn range(
        conn: &mut PgConnection,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealEntryRow>> {
        let sql = format!(
            "{ENTRY_SELECT} JOIN meal_slot s ON s.id = e.slot_id
             WHERE e.date BETWEEN $1 AND $2 ORDER BY e.date, s.ord, s.time, e.ord, e.id"
        );
        Ok(sqlx::query_as::<_, MealEntryRow>(&sql)
            .bind(from)
            .bind(to)
            .fetch_all(conn)
            .await?)
    }

    pub async fn get(conn: &mut PgConnection, id: MealPlanEntryId) -> Result<Option<MealEntryRow>> {
        let sql = format!("{ENTRY_SELECT} WHERE e.id = $1");
        Ok(sqlx::query_as::<_, MealEntryRow>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?)
    }

    pub async fn many(
        conn: &mut PgConnection,
        ids: &[MealPlanEntryId],
    ) -> Result<Vec<MealEntryRow>> {
        let ids: Vec<Uuid> = ids.iter().map(|i| i.as_uuid()).collect();
        let sql = format!("{ENTRY_SELECT} WHERE e.id = ANY ($1) ORDER BY e.date, e.ord, e.id");
        Ok(sqlx::query_as::<_, MealEntryRow>(&sql)
            .bind(&ids)
            .fetch_all(conn)
            .await?)
    }

    /// The entries of one slot on one day, in `ord` order.
    pub async fn entries_at(
        conn: &mut PgConnection,
        date: NaiveDate,
        slot: MealSlotId,
    ) -> Result<Vec<MealEntryRow>> {
        let sql =
            format!("{ENTRY_SELECT} WHERE e.date = $1 AND e.slot_id = $2 ORDER BY e.ord, e.id");
        Ok(sqlx::query_as::<_, MealEntryRow>(&sql)
            .bind(date)
            .bind(slot.as_uuid())
            .fetch_all(conn)
            .await?)
    }

    /// `max(ord) + 1` in the slot of that day, or 0 when it is empty.
    pub async fn next_ord(
        conn: &mut PgConnection,
        date: NaiveDate,
        slot: MealSlotId,
    ) -> Result<i32> {
        Ok(sqlx::query_scalar(
            "SELECT COALESCE(max(ord) + 1, 0) FROM meal_plan_entry WHERE date = $1 AND slot_id = $2",
        )
        .bind(date)
        .bind(slot.as_uuid())
        .fetch_one(conn)
        .await?)
    }

    /// Appends an entry to its slot (`ord` = max + 1; several entries per slot, D-120).
    pub async fn insert(
        conn: &mut PgConnection,
        household: HouseholdId,
        e: &NewEntry,
    ) -> Result<MealEntryRow> {
        let ord = Self::next_ord(conn, e.date, e.slot_id).await?;
        let id = MealPlanEntryId::new();
        let (recipe, text) = split(&e.content);
        sqlx::query(
            "INSERT INTO meal_plan_entry (id, household_id, date, slot_id, recipe_id, free_text,
                                          servings, created_by, ord)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
        )
        .bind(id.as_uuid())
        .bind(household.as_uuid())
        .bind(e.date)
        .bind(e.slot_id.as_uuid())
        .bind(recipe)
        .bind(text)
        .bind(e.servings)
        .bind(e.created_by.map(|m| m.as_uuid()))
        .bind(ord)
        .execute(&mut *conn)
        .await?;
        Self::get(conn, id).await?.ok_or(StoreError::NotFound)
    }

    /// Re-inserts a removed entry exactly as it was (id, `ord`, version): the undo of a delete.
    pub async fn restore(
        conn: &mut PgConnection,
        household: HouseholdId,
        row: &MealEntryRow,
    ) -> Result<()> {
        let content = match (row.recipe_id, &row.free_text) {
            (Some(r), _) => EntryContent::Recipe(r),
            (None, Some(t)) => EntryContent::FreeText(t.clone()),
            (None, None) => return Err(StoreError::Corrupt("entry without content".into())),
        };
        let (recipe, text) = split(&content);
        sqlx::query(
            "INSERT INTO meal_plan_entry (id, household_id, date, slot_id, recipe_id, free_text,
                                          servings, created_by, ord, version)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
             ON CONFLICT (id) DO NOTHING",
        )
        .bind(row.id.as_uuid())
        .bind(household.as_uuid())
        .bind(row.date)
        .bind(row.slot_id.as_uuid())
        .bind(recipe)
        .bind(text)
        .bind(row.servings)
        .bind(row.created_by.map(|m| m.as_uuid()))
        .bind(row.ord)
        .bind(row.version)
        .execute(conn)
        .await?;
        Ok(())
    }

    /// Applies the set fields. A new date or slot appends the entry there. Returns the new
    /// version, or `Conflict` if stale.
    pub async fn update(
        conn: &mut PgConnection,
        id: MealPlanEntryId,
        p: &EntryPatch,
        expected_version: i32,
    ) -> Result<i32> {
        let cur = Self::get(conn, id).await?.ok_or(StoreError::NotFound)?;
        let date = p.date.unwrap_or(cur.date);
        let slot = p.slot_id.unwrap_or(cur.slot_id);
        let ord = if date != cur.date || slot != cur.slot_id {
            Self::next_ord(conn, date, slot).await?
        } else {
            cur.ord
        };
        let (set_content, recipe, text) = match &p.content {
            Some(c) => {
                let (r, t) = split(c);
                (true, r, t)
            }
            None => (false, None, None),
        };
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE meal_plan_entry SET date = $2, slot_id = $3, ord = $4,
                    recipe_id = CASE WHEN $5 THEN $6 ELSE recipe_id END,
                    free_text = CASE WHEN $5 THEN $7 ELSE free_text END,
                    servings = CASE WHEN $8 THEN $9 ELSE servings END,
                    version = version + 1
             WHERE id = $1 AND version = $10
             RETURNING version",
        )
        .bind(id.as_uuid())
        .bind(date)
        .bind(slot.as_uuid())
        .bind(ord)
        .bind(set_content)
        .bind(recipe)
        .bind(text)
        .bind(p.servings.is_some())
        .bind(p.servings.flatten())
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "meal_plan_entry", id.as_uuid()).await),
        }
    }

    pub async fn delete(conn: &mut PgConnection, id: MealPlanEntryId) -> Result<()> {
        let n = sqlx::query("DELETE FROM meal_plan_entry WHERE id = $1")
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

    /// Deletes every entry of a slot on a day and returns them (template apply, "Replace").
    pub async fn delete_at(
        conn: &mut PgConnection,
        date: NaiveDate,
        slot: MealSlotId,
    ) -> Result<Vec<MealEntryRow>> {
        let rows = Self::entries_at(conn, date, slot).await?;
        sqlx::query("DELETE FROM meal_plan_entry WHERE date = $1 AND slot_id = $2")
            .bind(date)
            .bind(slot.as_uuid())
            .execute(conn)
            .await?;
        Ok(rows)
    }

    /// Exchanges `(date, slot_id, ord)` of two entries and bumps both versions.
    pub async fn swap(
        conn: &mut PgConnection,
        a: MealPlanEntryId,
        b: MealPlanEntryId,
    ) -> Result<()> {
        let ra = Self::get(conn, a).await?.ok_or(StoreError::NotFound)?;
        let rb = Self::get(conn, b).await?.ok_or(StoreError::NotFound)?;
        for (id, to) in [(a, &rb), (b, &ra)] {
            sqlx::query(
                "UPDATE meal_plan_entry SET date = $2, slot_id = $3, ord = $4, version = version + 1
                 WHERE id = $1",
            )
            .bind(id.as_uuid())
            .bind(to.date)
            .bind(to.slot_id.as_uuid())
            .bind(to.ord)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    /// The meals of one day in visible slots, in slot order (the dashboard panel and Today card).
    pub async fn day_meals(conn: &mut PgConnection, date: NaiveDate) -> Result<Vec<DayMealRow>> {
        Ok(sqlx::query_as::<_, DayMealRow>(
            "SELECT s.name AS slot_name, s.time AS slot_time, COALESCE(r.title, e.free_text, '') AS title,
                    e.recipe_id
             FROM meal_plan_entry e
             JOIN meal_slot s ON s.id = e.slot_id AND s.visible
             LEFT JOIN recipe r ON r.id = e.recipe_id
             WHERE e.date = $1 ORDER BY s.ord, s.time, e.ord, e.id",
        )
        .bind(date)
        .fetch_all(conn)
        .await?)
    }
}

// ---- templates ----

pub struct MealTemplateRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct MealTemplateRow {
    pub id: MealTemplateId,
    pub name: String,
    pub days: i32,
    pub note: Option<String>,
    pub version: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MealTemplateListRow {
    pub row: MealTemplateRow,
    /// The distinct slots the template uses, in slot order.
    pub slot_ids: Vec<MealSlotId>,
}

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct TemplateCellRow {
    pub day: i32,
    pub slot_id: MealSlotId,
    pub ord: i32,
    pub recipe_id: Option<RecipeId>,
    /// The recipe's title, or the free text.
    pub title: String,
    pub free_text: Option<String>,
    pub servings: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewTemplateCell {
    pub day: i32,
    pub slot_id: MealSlotId,
    pub ord: i32,
    pub content: EntryContent,
    pub servings: Option<i32>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NewTemplate {
    pub name: String,
    pub days: i32,
    pub note: Option<String>,
}

impl MealTemplateRepo {
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<MealTemplateListRow>> {
        let rows = sqlx::query_as::<_, MealTemplateRow>(
            "SELECT id, name, days, note, version FROM meal_template ORDER BY lower(name), id",
        )
        .fetch_all(&mut *conn)
        .await?;
        let mut out = Vec::with_capacity(rows.len());
        for row in rows {
            let slots: Vec<Uuid> = sqlx::query_scalar(
                "SELECT s.id FROM meal_slot s
                 WHERE EXISTS (SELECT 1 FROM meal_template_cell c WHERE c.template_id = $1 AND c.slot_id = s.id)
                 ORDER BY s.ord, s.time, s.id",
            )
            .bind(row.id.as_uuid())
            .fetch_all(&mut *conn)
            .await?;
            out.push(MealTemplateListRow {
                row,
                slot_ids: slots.into_iter().map(Into::into).collect(),
            });
        }
        Ok(out)
    }

    pub async fn get(
        conn: &mut PgConnection,
        id: MealTemplateId,
    ) -> Result<Option<MealTemplateRow>> {
        Ok(sqlx::query_as::<_, MealTemplateRow>(
            "SELECT id, name, days, note, version FROM meal_template WHERE id = $1",
        )
        .bind(id.as_uuid())
        .fetch_optional(conn)
        .await?)
    }

    pub async fn cells(
        conn: &mut PgConnection,
        id: MealTemplateId,
    ) -> Result<Vec<TemplateCellRow>> {
        Ok(sqlx::query_as::<_, TemplateCellRow>(
            "SELECT c.day, c.slot_id, c.ord, c.recipe_id, COALESCE(r.title, c.free_text, '') AS title,
                    c.free_text, c.servings
             FROM meal_template_cell c
             JOIN meal_slot s ON s.id = c.slot_id
             LEFT JOIN recipe r ON r.id = c.recipe_id
             WHERE c.template_id = $1 ORDER BY c.day, s.ord, s.time, c.ord, c.id",
        )
        .bind(id.as_uuid())
        .fetch_all(conn)
        .await?)
    }

    pub async fn get_with_cells(
        conn: &mut PgConnection,
        id: MealTemplateId,
    ) -> Result<Option<(MealTemplateRow, Vec<TemplateCellRow>)>> {
        let Some(row) = Self::get(conn, id).await? else {
            return Ok(None);
        };
        let cells = Self::cells(conn, id).await?;
        Ok(Some((row, cells)))
    }

    async fn insert_cells(
        conn: &mut PgConnection,
        household: HouseholdId,
        template: MealTemplateId,
        cells: &[NewTemplateCell],
    ) -> Result<()> {
        for c in cells {
            let (recipe, text) = split(&c.content);
            sqlx::query(
                "INSERT INTO meal_template_cell (id, household_id, template_id, day, slot_id, recipe_id,
                                                 free_text, servings, ord)
                 VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            )
            .bind(Uuid::now_v7())
            .bind(household.as_uuid())
            .bind(template.as_uuid())
            .bind(c.day)
            .bind(c.slot_id.as_uuid())
            .bind(recipe)
            .bind(text)
            .bind(c.servings)
            .bind(c.ord)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }

    pub async fn insert(
        conn: &mut PgConnection,
        household: HouseholdId,
        t: &NewTemplate,
        cells: &[NewTemplateCell],
    ) -> Result<MealTemplateRow> {
        let id = MealTemplateId::new();
        sqlx::query(
            "INSERT INTO meal_template (id, household_id, name, days, note) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(id.as_uuid())
        .bind(household.as_uuid())
        .bind(&t.name)
        .bind(t.days)
        .bind(&t.note)
        .execute(&mut *conn)
        .await?;
        Self::insert_cells(conn, household, id, cells).await?;
        Self::get(conn, id).await?.ok_or(StoreError::NotFound)
    }

    /// Writes the header and replaces the cells whole. Returns the new version, or `Conflict`.
    pub async fn update_with_cells(
        conn: &mut PgConnection,
        id: MealTemplateId,
        t: &NewTemplate,
        cells: &[NewTemplateCell],
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<(i32, Uuid)> = sqlx::query_as(
            "UPDATE meal_template SET name = $2, days = $3, note = $4, version = version + 1
             WHERE id = $1 AND version = $5 RETURNING version, household_id",
        )
        .bind(id.as_uuid())
        .bind(&t.name)
        .bind(t.days)
        .bind(&t.note)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        let Some((version, household)) = v else {
            return Err(conflict_or_missing(conn, "meal_template", id.as_uuid()).await);
        };
        sqlx::query("DELETE FROM meal_template_cell WHERE template_id = $1")
            .bind(id.as_uuid())
            .execute(&mut *conn)
            .await?;
        Self::insert_cells(conn, household.into(), id, cells).await?;
        Ok(version)
    }

    pub async fn delete(conn: &mut PgConnection, id: MealTemplateId) -> Result<()> {
        let n = sqlx::query("DELETE FROM meal_template WHERE id = $1")
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

    /// Copies a template and its cells under a new name; later edits to either leave the other alone.
    pub async fn duplicate(
        conn: &mut PgConnection,
        id: MealTemplateId,
        new_name: &str,
    ) -> Result<MealTemplateRow> {
        let src: Option<(Uuid, i32, Option<String>)> =
            sqlx::query_as("SELECT household_id, days, note FROM meal_template WHERE id = $1")
                .bind(id.as_uuid())
                .fetch_optional(&mut *conn)
                .await?;
        let (household, days, note) = src.ok_or(StoreError::NotFound)?;
        let new_id = MealTemplateId::new();
        sqlx::query(
            "INSERT INTO meal_template (id, household_id, name, days, note) VALUES ($1, $2, $3, $4, $5)",
        )
        .bind(new_id.as_uuid())
        .bind(household)
        .bind(new_name)
        .bind(days)
        .bind(note)
        .execute(&mut *conn)
        .await?;
        sqlx::query(
            "INSERT INTO meal_template_cell (id, household_id, template_id, day, slot_id, recipe_id,
                                             free_text, servings, ord)
             SELECT gen_random_uuid(), household_id, $2, day, slot_id, recipe_id, free_text, servings, ord
             FROM meal_template_cell WHERE template_id = $1",
        )
        .bind(id.as_uuid())
        .bind(new_id.as_uuid())
        .execute(&mut *conn)
        .await?;
        Self::get(conn, new_id).await?.ok_or(StoreError::NotFound)
    }
}
