//! M5 repos: recipes, their image, lines, steps, ratings and the ingredient catalogue.

use chrono::{DateTime, NaiveDate, Utc};
use fa_domain::{
    BaseUnit, HouseholdId, ImportMethod, IngredientId, IngredientOrigin, MatchState, MemberId,
    RecipeId, RecipeLineId,
};
use serde_json::Value;
use sqlx::{FromRow, PgConnection};
use uuid::Uuid;

use super::{conflict_or_missing, enum_str, parse_enum};
use crate::{Result, StoreError};

// ---- recipe ----

pub struct RecipeRepo;

/// The editable columns of a recipe.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RecipeFields {
    pub title: String,
    pub source_url: Option<String>,
    pub servings: Option<i32>,
    pub yield_text: Option<String>,
    pub prep_minutes: Option<i32>,
    pub cook_minutes: Option<i32>,
    pub total_minutes: Option<i32>,
    pub tags: Vec<String>,
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RecipeRow {
    pub id: RecipeId,
    pub household_id: HouseholdId,
    pub fields: RecipeFields,
    pub import_method: ImportMethod,
    pub favourite: bool,
    pub created_by: Option<MemberId>,
    pub has_image: bool,
    pub version: i32,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(FromRow)]
struct RawRecipe {
    id: Uuid,
    household_id: Uuid,
    title: String,
    source_url: Option<String>,
    servings: Option<i32>,
    yield_text: Option<String>,
    prep_minutes: Option<i32>,
    cook_minutes: Option<i32>,
    total_minutes: Option<i32>,
    import_method: String,
    tags: Vec<String>,
    note: Option<String>,
    favourite: bool,
    created_by: Option<Uuid>,
    has_image: bool,
    version: i32,
    created_at: DateTime<Utc>,
    updated_at: DateTime<Utc>,
}

impl TryFrom<RawRecipe> for RecipeRow {
    type Error = StoreError;
    fn try_from(r: RawRecipe) -> Result<Self> {
        Ok(RecipeRow {
            id: r.id.into(),
            household_id: r.household_id.into(),
            fields: RecipeFields {
                title: r.title,
                source_url: r.source_url,
                servings: r.servings,
                yield_text: r.yield_text,
                prep_minutes: r.prep_minutes,
                cook_minutes: r.cook_minutes,
                total_minutes: r.total_minutes,
                tags: r.tags,
                note: r.note,
            },
            import_method: parse_enum(&r.import_method)?,
            favourite: r.favourite,
            created_by: r.created_by.map(Into::into),
            has_image: r.has_image,
            version: r.version,
            created_at: r.created_at,
            updated_at: r.updated_at,
        })
    }
}

const RECIPE_COLS: &str = "r.id, r.household_id, r.title, r.source_url, r.servings, r.yield_text,
    r.prep_minutes, r.cook_minutes, r.total_minutes, r.import_method, r.tags, r.note, r.favourite,
    r.created_by, EXISTS (SELECT 1 FROM recipe_image i WHERE i.recipe_id = r.id) AS has_image,
    r.version, r.created_at, r.updated_at";

/// One row of the library list.
#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct RecipeListRow {
    pub id: RecipeId,
    pub title: String,
    /// `lower(title)` as the database computes it: the first half of the paging cursor.
    pub title_key: String,
    pub has_image: bool,
    pub total_minutes: Option<i32>,
    pub tags: Vec<String>,
    pub favourite: bool,
    pub rating_avg: Option<f32>,
    pub my_rating: Option<i16>,
    pub last_planned: Option<NaiveDate>,
}

/// Filters and keyset paging for [`RecipeRepo::list`]. Order is `(lower(title), id)`.
#[derive(Debug, Clone, Default)]
pub struct RecipeQuery {
    /// Matches the title, or an ingredient (cleaned item or catalogue name), case-insensitively.
    pub q: Option<String>,
    pub tag: Option<String>,
    pub favourite: Option<bool>,
    /// For `my_rating`.
    pub me: Option<MemberId>,
    /// The household's calendar day: `last_planned` is the latest planned date on or before it.
    pub today: Option<NaiveDate>,
    /// Keyset cursor: `(title_key, id)` of the last row of the previous page.
    pub after: Option<(String, RecipeId)>,
    pub limit: i64,
}

impl RecipeRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        id: RecipeId,
        household: HouseholdId,
        f: &RecipeFields,
        import_method: ImportMethod,
        created_by: Option<MemberId>,
    ) -> Result<RecipeRow> {
        sqlx::query(
            "INSERT INTO recipe (id, household_id, title, source_url, servings, yield_text,
                                 prep_minutes, cook_minutes, total_minutes, import_method, tags,
                                 note, created_by)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)",
        )
        .bind(id.as_uuid())
        .bind(household.as_uuid())
        .bind(&f.title)
        .bind(&f.source_url)
        .bind(f.servings)
        .bind(&f.yield_text)
        .bind(f.prep_minutes)
        .bind(f.cook_minutes)
        .bind(f.total_minutes)
        .bind(enum_str(&import_method))
        .bind(&f.tags)
        .bind(&f.note)
        .bind(created_by.map(|m| m.as_uuid()))
        .execute(&mut *conn)
        .await?;
        Self::get(conn, id).await?.ok_or(StoreError::NotFound)
    }

    pub async fn get(conn: &mut PgConnection, id: RecipeId) -> Result<Option<RecipeRow>> {
        let sql = format!("SELECT {RECIPE_COLS} FROM recipe r WHERE r.id = $1");
        let row = sqlx::query_as::<_, RawRecipe>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Updates the editable columns. Returns the new version, or `Conflict` if stale.
    pub async fn update(
        conn: &mut PgConnection,
        id: RecipeId,
        f: &RecipeFields,
        expected_version: i32,
    ) -> Result<i32> {
        let v: Option<i32> = sqlx::query_scalar(
            "UPDATE recipe SET title = $2, source_url = $3, servings = $4, yield_text = $5,
                    prep_minutes = $6, cook_minutes = $7, total_minutes = $8, tags = $9, note = $10,
                    version = version + 1
             WHERE id = $1 AND version = $11
             RETURNING version",
        )
        .bind(id.as_uuid())
        .bind(&f.title)
        .bind(&f.source_url)
        .bind(f.servings)
        .bind(&f.yield_text)
        .bind(f.prep_minutes)
        .bind(f.cook_minutes)
        .bind(f.total_minutes)
        .bind(&f.tags)
        .bind(&f.note)
        .bind(expected_version)
        .fetch_optional(&mut *conn)
        .await?;
        match v {
            Some(v) => Ok(v),
            None => Err(conflict_or_missing(conn, "recipe", id.as_uuid()).await),
        }
    }

    /// The favourite flag is instant: no version check, no version bump.
    pub async fn set_favourite(conn: &mut PgConnection, id: RecipeId, on: bool) -> Result<()> {
        let n = sqlx::query("UPDATE recipe SET favourite = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(on)
            .execute(conn)
            .await?
            .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Deletes the recipe. Plan entries that use it keep the title as free text (the entry CHECK
    /// needs both changes in one statement, so the FK's SET NULL alone would fail).
    pub async fn delete(conn: &mut PgConnection, id: RecipeId) -> Result<()> {
        sqlx::query(
            "UPDATE meal_plan_entry e SET recipe_id = NULL, free_text = r.title, version = e.version + 1
             FROM recipe r WHERE e.recipe_id = r.id AND r.id = $1",
        )
        .bind(id.as_uuid())
        .execute(&mut *conn)
        .await?;
        let n = sqlx::query("DELETE FROM recipe WHERE id = $1")
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

    /// The library list, ordered by `(lower(title), id)`. Fetches `limit` rows; ask for one more
    /// than the page size to know whether there is a next page.
    pub async fn list(conn: &mut PgConnection, q: &RecipeQuery) -> Result<Vec<RecipeListRow>> {
        let (after_title, after_id) = match &q.after {
            Some((t, i)) => (Some(t.clone()), Some(i.as_uuid())),
            None => (None, None),
        };
        let rows = sqlx::query_as::<_, RecipeListRow>(
            "SELECT r.id, r.title, lower(r.title) AS title_key,
                    EXISTS (SELECT 1 FROM recipe_image i WHERE i.recipe_id = r.id) AS has_image,
                    r.total_minutes, r.tags, r.favourite,
                    (SELECT avg(stars)::real FROM recipe_rating x WHERE x.recipe_id = r.id) AS rating_avg,
                    (SELECT stars FROM recipe_rating x WHERE x.recipe_id = r.id AND x.member_id = $1) AS my_rating,
                    (SELECT max(e.date) FROM meal_plan_entry e
                      WHERE e.recipe_id = r.id AND ($2::date IS NULL OR e.date <= $2)) AS last_planned
             FROM recipe r
             WHERE ($3::text IS NULL
                    OR strpos(lower(r.title), lower($3)) > 0
                    OR EXISTS (SELECT 1 FROM recipe_ingredient ri
                               LEFT JOIN ingredient g ON g.id = ri.ingredient_id
                               WHERE ri.recipe_id = r.id
                                 AND (strpos(lower(coalesce(ri.item, '')), lower($3)) > 0
                                      OR strpos(lower(coalesce(g.canonical_name, '')), lower($3)) > 0)))
               AND ($4::text IS NULL OR $4 = ANY (r.tags))
               AND ($5::boolean IS NULL OR r.favourite = $5)
               AND ($6::text IS NULL OR (lower(r.title), r.id) > ($6, $7))
             ORDER BY lower(r.title), r.id
             LIMIT $8",
        )
        .bind(q.me.map(|m| m.as_uuid()))
        .bind(q.today)
        .bind(q.q.as_deref().filter(|s| !s.is_empty()))
        .bind(&q.tag)
        .bind(q.favourite)
        .bind(after_title)
        .bind(after_id)
        .bind(q.limit.max(1))
        .fetch_all(conn)
        .await?;
        Ok(rows)
    }

    /// Every distinct tag in the library, sorted (for the filter chips).
    pub async fn all_tags(conn: &mut PgConnection) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT DISTINCT unnest(tags) AS t FROM recipe ORDER BY t")
                .fetch_all(conn)
                .await?,
        )
    }

    pub async fn count(conn: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM recipe")
            .fetch_one(conn)
            .await?)
    }
}

// ---- image ----

pub struct RecipeImageRepo;

#[derive(Debug, Clone, PartialEq, FromRow)]
pub struct RecipeImageRow {
    pub mime: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
    pub source: String,
}

impl RecipeImageRepo {
    /// Inserts or replaces the recipe's image (`source` is `import` or `upload`).
    pub async fn put(
        conn: &mut PgConnection,
        recipe: RecipeId,
        mime: &str,
        bytes: &[u8],
        sha256: &str,
        source: &str,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO recipe_image (recipe_id, household_id, mime, bytes, sha256, source)
             SELECT id, household_id, $2, $3, $4, $5 FROM recipe WHERE id = $1
             ON CONFLICT (recipe_id) DO UPDATE
                SET mime = EXCLUDED.mime, bytes = EXCLUDED.bytes, sha256 = EXCLUDED.sha256,
                    source = EXCLUDED.source",
        )
        .bind(recipe.as_uuid())
        .bind(mime)
        .bind(bytes)
        .bind(sha256)
        .bind(source)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn get(conn: &mut PgConnection, recipe: RecipeId) -> Result<Option<RecipeImageRow>> {
        Ok(sqlx::query_as::<_, RecipeImageRow>(
            "SELECT mime, bytes, sha256, source FROM recipe_image WHERE recipe_id = $1",
        )
        .bind(recipe.as_uuid())
        .fetch_optional(conn)
        .await?)
    }

    /// The stored sha256 only (for `If-None-Match` and the import job's "already stored" check).
    pub async fn sha(conn: &mut PgConnection, recipe: RecipeId) -> Result<Option<String>> {
        Ok(
            sqlx::query_scalar("SELECT sha256 FROM recipe_image WHERE recipe_id = $1")
                .bind(recipe.as_uuid())
                .fetch_optional(conn)
                .await?,
        )
    }

    /// Returns whether an image was removed.
    pub async fn delete(conn: &mut PgConnection, recipe: RecipeId) -> Result<bool> {
        Ok(sqlx::query("DELETE FROM recipe_image WHERE recipe_id = $1")
            .bind(recipe.as_uuid())
            .execute(conn)
            .await?
            .rows_affected()
            > 0)
    }
}

// ---- lines ----

pub struct RecipeLineRepo;

#[derive(Debug, Clone, PartialEq)]
pub struct RecipeLineRow {
    pub id: RecipeLineId,
    pub recipe_id: RecipeId,
    pub ord: i32,
    pub raw_text: String,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub item: Option<String>,
    pub note: Option<String>,
    pub ingredient_id: Option<IngredientId>,
    /// The linked catalogue entry's canonical name.
    pub ingredient_name: Option<String>,
    pub match_state: MatchState,
    pub proposal: Option<Value>,
}

#[derive(FromRow)]
struct RawLine {
    id: Uuid,
    recipe_id: Uuid,
    ord: i32,
    raw_text: String,
    qty: Option<f64>,
    unit: Option<String>,
    item: Option<String>,
    note: Option<String>,
    ingredient_id: Option<Uuid>,
    ingredient_name: Option<String>,
    match_state: String,
    proposal: Option<Value>,
}

impl TryFrom<RawLine> for RecipeLineRow {
    type Error = StoreError;
    fn try_from(r: RawLine) -> Result<Self> {
        Ok(RecipeLineRow {
            id: r.id.into(),
            recipe_id: r.recipe_id.into(),
            ord: r.ord,
            raw_text: r.raw_text,
            qty: r.qty,
            unit: r.unit,
            item: r.item,
            note: r.note,
            ingredient_id: r.ingredient_id.map(Into::into),
            ingredient_name: r.ingredient_name,
            match_state: parse_enum(&r.match_state)?,
            proposal: r.proposal,
        })
    }
}

const LINE_SELECT: &str =
    "SELECT l.id, l.recipe_id, l.ord, l.raw_text, l.qty, l.unit, l.item, l.note,
        l.ingredient_id, i.canonical_name AS ingredient_name, l.match_state, l.proposal
     FROM recipe_ingredient l LEFT JOIN ingredient i ON i.id = l.ingredient_id";

/// A parsed line as the normaliser hands it over (the position is the slice index).
#[derive(Debug, Clone, PartialEq)]
pub struct NewLine {
    pub raw_text: String,
    pub qty: Option<f64>,
    pub unit: Option<String>,
    pub item: Option<String>,
    pub note: Option<String>,
    pub ingredient_id: Option<IngredientId>,
    pub match_state: MatchState,
    pub proposal: Option<Value>,
}

/// One line after [`RecipeLineRepo::replace_for_recipe`].
#[derive(Debug, Clone, PartialEq)]
pub struct ReplacedLine {
    pub row: RecipeLineRow,
    /// True when an existing line with the same `raw_text` was kept (its id, `ingredient_id`,
    /// `match_state` and `proposal` survive; the parsed columns come from the new line).
    pub kept: bool,
}

impl RecipeLineRepo {
    pub async fn for_recipe(
        conn: &mut PgConnection,
        recipe: RecipeId,
    ) -> Result<Vec<RecipeLineRow>> {
        let sql = format!("{LINE_SELECT} WHERE l.recipe_id = $1 ORDER BY l.ord");
        let rows = sqlx::query_as::<_, RawLine>(&sql)
            .bind(recipe.as_uuid())
            .fetch_all(conn)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }

    pub async fn get(conn: &mut PgConnection, id: RecipeLineId) -> Result<Option<RecipeLineRow>> {
        let sql = format!("{LINE_SELECT} WHERE l.id = $1");
        let row = sqlx::query_as::<_, RawLine>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Replaces the recipe's lines whole. A new line whose `raw_text` equals an existing, not yet
    /// reused line keeps that line's id and match; every other new line is inserted, and old
    /// lines nobody reused are deleted.
    pub async fn replace_for_recipe(
        conn: &mut PgConnection,
        recipe: RecipeId,
        lines: &[NewLine],
    ) -> Result<Vec<ReplacedLine>> {
        let household: Option<Uuid> =
            sqlx::query_scalar("SELECT household_id FROM recipe WHERE id = $1")
                .bind(recipe.as_uuid())
                .fetch_optional(&mut *conn)
                .await?;
        let household = household.ok_or(StoreError::NotFound)?;
        let existing = Self::for_recipe(conn, recipe).await?;
        // Park the old positions so the (recipe_id, ord) unique key never collides mid-way.
        sqlx::query("UPDATE recipe_ingredient SET ord = -ord - 1 WHERE recipe_id = $1")
            .bind(recipe.as_uuid())
            .execute(&mut *conn)
            .await?;
        let mut used = vec![false; existing.len()];
        let mut out = Vec::with_capacity(lines.len());
        for (ord, l) in lines.iter().enumerate() {
            let ord =
                i32::try_from(ord).map_err(|_| StoreError::Corrupt("too many lines".into()))?;
            let hit = existing
                .iter()
                .enumerate()
                .find(|(i, e)| !used[*i] && e.raw_text == l.raw_text)
                .map(|(i, _)| i);
            let id = match hit {
                Some(i) => {
                    used[i] = true;
                    let id = existing[i].id;
                    sqlx::query(
                        "UPDATE recipe_ingredient SET ord = $2, qty = $3, unit = $4, item = $5, note = $6
                         WHERE id = $1",
                    )
                    .bind(id.as_uuid())
                    .bind(ord)
                    .bind(l.qty)
                    .bind(&l.unit)
                    .bind(&l.item)
                    .bind(&l.note)
                    .execute(&mut *conn)
                    .await?;
                    id
                }
                None => {
                    let id = RecipeLineId::new();
                    sqlx::query(
                        "INSERT INTO recipe_ingredient (id, household_id, recipe_id, ord, raw_text, qty,
                                unit, item, note, ingredient_id, match_state, proposal)
                         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12)",
                    )
                    .bind(id.as_uuid())
                    .bind(household)
                    .bind(recipe.as_uuid())
                    .bind(ord)
                    .bind(&l.raw_text)
                    .bind(l.qty)
                    .bind(&l.unit)
                    .bind(&l.item)
                    .bind(&l.note)
                    .bind(l.ingredient_id.map(|i| i.as_uuid()))
                    .bind(enum_str(&l.match_state))
                    .bind(&l.proposal)
                    .execute(&mut *conn)
                    .await?;
                    id
                }
            };
            out.push((id, hit.is_some()));
        }
        sqlx::query("DELETE FROM recipe_ingredient WHERE recipe_id = $1 AND ord < 0")
            .bind(recipe.as_uuid())
            .execute(&mut *conn)
            .await?;
        let rows = Self::for_recipe(conn, recipe).await?;
        let mut result = Vec::with_capacity(out.len());
        for (id, kept) in out {
            let row = rows
                .iter()
                .find(|r| r.id == id)
                .cloned()
                .ok_or_else(|| StoreError::Corrupt("replaced line vanished".into()))?;
            result.push(ReplacedLine { row, kept });
        }
        Ok(result)
    }

    /// Sets a line's match (the match route, the normaliser and `ingredient.llm`).
    pub async fn set_match(
        conn: &mut PgConnection,
        id: RecipeLineId,
        ingredient: Option<IngredientId>,
        state: MatchState,
        proposal: Option<&Value>,
    ) -> Result<()> {
        let n = sqlx::query(
            "UPDATE recipe_ingredient SET ingredient_id = $2, match_state = $3, proposal = $4
             WHERE id = $1",
        )
        .bind(id.as_uuid())
        .bind(ingredient.map(|i| i.as_uuid()))
        .bind(enum_str(&state))
        .bind(proposal)
        .execute(conn)
        .await?
        .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Lines still unmatched (oldest first), for the LLM matcher's backlog.
    pub async fn unmatched(conn: &mut PgConnection, limit: i64) -> Result<Vec<RecipeLineRow>> {
        let sql = format!(
            "{LINE_SELECT} WHERE l.match_state = 'unmatched' AND l.item IS NOT NULL
             ORDER BY l.created_at, l.id LIMIT $1"
        );
        let rows = sqlx::query_as::<_, RawLine>(&sql)
            .bind(limit)
            .fetch_all(conn)
            .await?;
        rows.into_iter().map(TryInto::try_into).collect()
    }
}

// ---- steps ----

pub struct RecipeStepRepo;

impl RecipeStepRepo {
    pub async fn for_recipe(conn: &mut PgConnection, recipe: RecipeId) -> Result<Vec<String>> {
        Ok(
            sqlx::query_scalar("SELECT text FROM recipe_step WHERE recipe_id = $1 ORDER BY ord")
                .bind(recipe.as_uuid())
                .fetch_all(conn)
                .await?,
        )
    }

    pub async fn replace_for_recipe(
        conn: &mut PgConnection,
        recipe: RecipeId,
        steps: &[String],
    ) -> Result<()> {
        sqlx::query("DELETE FROM recipe_step WHERE recipe_id = $1")
            .bind(recipe.as_uuid())
            .execute(&mut *conn)
            .await?;
        for (ord, text) in steps.iter().enumerate() {
            sqlx::query(
                "INSERT INTO recipe_step (id, household_id, recipe_id, ord, text)
                 SELECT $1, household_id, id, $3, $4 FROM recipe WHERE id = $2",
            )
            .bind(Uuid::now_v7())
            .bind(recipe.as_uuid())
            .bind(i32::try_from(ord).map_err(|_| StoreError::Corrupt("too many steps".into()))?)
            .bind(text)
            .execute(&mut *conn)
            .await?;
        }
        Ok(())
    }
}

// ---- ratings ----

pub struct RecipeRatingRepo;

impl RecipeRatingRepo {
    pub async fn set(
        conn: &mut PgConnection,
        recipe: RecipeId,
        member: MemberId,
        stars: i16,
    ) -> Result<()> {
        sqlx::query(
            "INSERT INTO recipe_rating (recipe_id, member_id, household_id, stars)
             SELECT id, $2, household_id, $3 FROM recipe WHERE id = $1
             ON CONFLICT (recipe_id, member_id) DO UPDATE SET stars = EXCLUDED.stars",
        )
        .bind(recipe.as_uuid())
        .bind(member.as_uuid())
        .bind(stars)
        .execute(conn)
        .await?;
        Ok(())
    }

    pub async fn clear(conn: &mut PgConnection, recipe: RecipeId, member: MemberId) -> Result<()> {
        sqlx::query("DELETE FROM recipe_rating WHERE recipe_id = $1 AND member_id = $2")
            .bind(recipe.as_uuid())
            .bind(member.as_uuid())
            .execute(conn)
            .await?;
        Ok(())
    }

    /// Every member's stars, ordered by member id.
    pub async fn for_recipe(
        conn: &mut PgConnection,
        recipe: RecipeId,
    ) -> Result<Vec<(MemberId, i16)>> {
        let rows: Vec<(Uuid, i16)> = sqlx::query_as(
            "SELECT member_id, stars FROM recipe_rating WHERE recipe_id = $1 ORDER BY member_id",
        )
        .bind(recipe.as_uuid())
        .fetch_all(conn)
        .await?;
        Ok(rows.into_iter().map(|(m, s)| (m.into(), s)).collect())
    }
}

// ---- ingredients ----

pub struct IngredientRepo;

#[derive(Debug, Clone, PartialEq)]
pub struct IngredientRow {
    pub id: IngredientId,
    pub household_id: HouseholdId,
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub is_staple: bool,
    pub aisle_id: Option<fa_domain::AisleId>,
    pub default_store_id: Option<fa_domain::GroceryStoreId>,
    pub default_unit: Option<BaseUnit>,
    pub origin: IngredientOrigin,
}

/// An ingredient to insert or to write over an existing one (`origin` is ignored on update).
#[derive(Debug, Clone, PartialEq)]
pub struct NewIngredient {
    pub canonical_name: String,
    pub aliases: Vec<String>,
    pub is_staple: bool,
    pub aisle_id: Option<fa_domain::AisleId>,
    pub default_store_id: Option<fa_domain::GroceryStoreId>,
    pub default_unit: Option<BaseUnit>,
    pub origin: IngredientOrigin,
}

#[derive(FromRow)]
struct RawIngredient {
    id: Uuid,
    household_id: Uuid,
    canonical_name: String,
    aliases: Vec<String>,
    is_staple: bool,
    aisle_id: Option<Uuid>,
    default_store_id: Option<Uuid>,
    default_unit: Option<String>,
    origin: String,
}

impl TryFrom<RawIngredient> for IngredientRow {
    type Error = StoreError;
    fn try_from(r: RawIngredient) -> Result<Self> {
        Ok(IngredientRow {
            id: r.id.into(),
            household_id: r.household_id.into(),
            canonical_name: r.canonical_name,
            aliases: r.aliases,
            is_staple: r.is_staple,
            aisle_id: r.aisle_id.map(Into::into),
            default_store_id: r.default_store_id.map(Into::into),
            default_unit: r.default_unit.as_deref().map(parse_enum).transpose()?,
            origin: parse_enum(&r.origin)?,
        })
    }
}

const ING_COLS: &str = "id, household_id, canonical_name, aliases, is_staple, aisle_id,
    default_store_id, default_unit, origin";

fn rows_into<R, T>(rows: Vec<R>) -> Result<Vec<T>>
where
    T: TryFrom<R, Error = StoreError>,
{
    rows.into_iter().map(TryInto::try_into).collect()
}

impl IngredientRepo {
    pub async fn insert(
        conn: &mut PgConnection,
        household: HouseholdId,
        n: &NewIngredient,
    ) -> Result<IngredientRow> {
        let sql = format!(
            "INSERT INTO ingredient (id, household_id, canonical_name, aliases, is_staple, aisle_id,
                                     default_store_id, default_unit, origin)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9) RETURNING {ING_COLS}"
        );
        let row = sqlx::query_as::<_, RawIngredient>(&sql)
            .bind(IngredientId::new().as_uuid())
            .bind(household.as_uuid())
            .bind(&n.canonical_name)
            .bind(&n.aliases)
            .bind(n.is_staple)
            .bind(n.aisle_id.map(|a| a.as_uuid()))
            .bind(n.default_store_id.map(|a| a.as_uuid()))
            .bind(n.default_unit.as_ref().map(enum_str))
            .bind(enum_str(&n.origin))
            .fetch_one(conn)
            .await?;
        row.try_into()
    }

    /// Bulk insert (the seed). Names that already exist are skipped. Returns the rows inserted.
    pub async fn insert_many(
        conn: &mut PgConnection,
        household: HouseholdId,
        items: &[NewIngredient],
    ) -> Result<u64> {
        if items.is_empty() {
            return Ok(0);
        }
        let json: Vec<Value> = items
            .iter()
            .map(|n| {
                serde_json::json!({
                    "id": IngredientId::new().as_uuid(),
                    "canonical_name": n.canonical_name,
                    "aliases": n.aliases,
                    "is_staple": n.is_staple,
                    "aisle_id": n.aisle_id.map(|a| a.as_uuid()),
                    "default_store_id": n.default_store_id.map(|a| a.as_uuid()),
                    "default_unit": n.default_unit.as_ref().map(enum_str),
                    "origin": enum_str(&n.origin),
                })
            })
            .collect();
        let done = sqlx::query(
            "INSERT INTO ingredient (id, household_id, canonical_name, aliases, is_staple, aisle_id,
                                     default_store_id, default_unit, origin)
             SELECT r.id, $1, r.canonical_name,
                    ARRAY(SELECT jsonb_array_elements_text(r.aliases)),
                    r.is_staple, r.aisle_id, r.default_store_id, r.default_unit, r.origin
             FROM jsonb_to_recordset($2::jsonb) AS r(id uuid, canonical_name text, aliases jsonb,
                    is_staple boolean, aisle_id uuid, default_store_id uuid, default_unit text,
                    origin text)
             ON CONFLICT (household_id, canonical_name) DO NOTHING",
        )
        .bind(household.as_uuid())
        .bind(Value::Array(json))
        .execute(conn)
        .await?;
        Ok(done.rows_affected())
    }

    pub async fn get(conn: &mut PgConnection, id: IngredientId) -> Result<Option<IngredientRow>> {
        let sql = format!("SELECT {ING_COLS} FROM ingredient WHERE id = $1");
        let row = sqlx::query_as::<_, RawIngredient>(&sql)
            .bind(id.as_uuid())
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// The whole catalogue, by name.
    pub async fn list(conn: &mut PgConnection) -> Result<Vec<IngredientRow>> {
        let sql = format!("SELECT {ING_COLS} FROM ingredient ORDER BY lower(canonical_name), id");
        rows_into(
            sqlx::query_as::<_, RawIngredient>(&sql)
                .fetch_all(conn)
                .await?,
        )
    }

    /// For the add-box suggestions: name or alias contains `q`, or the name is trigram-similar
    /// (`threshold`). Best matches first.
    pub async fn search(
        conn: &mut PgConnection,
        q: &str,
        threshold: f32,
        limit: i64,
    ) -> Result<Vec<IngredientRow>> {
        let sql = format!(
            "SELECT {ING_COLS} FROM ingredient
             WHERE strpos(lower(canonical_name), lower($1)) > 0
                OR EXISTS (SELECT 1 FROM unnest(aliases) a WHERE strpos(lower(a), lower($1)) > 0)
                OR similarity(canonical_name, $1) >= $2
             ORDER BY (lower(canonical_name) = lower($1)) DESC,
                      (strpos(lower(canonical_name), lower($1)) = 1) DESC,
                      similarity(canonical_name, $1) DESC, lower(canonical_name)
             LIMIT $3"
        );
        rows_into(
            sqlx::query_as::<_, RawIngredient>(&sql)
                .bind(q)
                .bind(threshold)
                .bind(limit)
                .fetch_all(conn)
                .await?,
        )
    }

    /// Case-insensitive match on the canonical name.
    pub async fn match_exact(conn: &mut PgConnection, name: &str) -> Result<Option<IngredientRow>> {
        let sql = format!(
            "SELECT {ING_COLS} FROM ingredient WHERE lower(canonical_name) = lower($1)
             ORDER BY id LIMIT 1"
        );
        let row = sqlx::query_as::<_, RawIngredient>(&sql)
            .bind(name)
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// Case-insensitive match on any alias.
    pub async fn match_alias(conn: &mut PgConnection, name: &str) -> Result<Option<IngredientRow>> {
        let sql = format!(
            "SELECT {ING_COLS} FROM ingredient
             WHERE EXISTS (SELECT 1 FROM unnest(aliases) a WHERE lower(a) = lower($1))
             ORDER BY lower(canonical_name), id LIMIT 1"
        );
        let row = sqlx::query_as::<_, RawIngredient>(&sql)
            .bind(name)
            .fetch_optional(conn)
            .await?;
        row.map(TryInto::try_into).transpose()
    }

    /// The closest canonical names by `pg_trgm` similarity, best first, with the score (0 to 1).
    /// The caller applies the household's threshold.
    pub async fn nearest(
        conn: &mut PgConnection,
        name: &str,
        limit: i64,
    ) -> Result<Vec<(IngredientId, String, f32)>> {
        let rows: Vec<(Uuid, String, f32)> = sqlx::query_as(
            "SELECT id, canonical_name, similarity(canonical_name, $1) AS score
             FROM ingredient
             ORDER BY score DESC, lower(canonical_name), id
             LIMIT $2",
        )
        .bind(name)
        .bind(limit)
        .fetch_all(conn)
        .await?;
        Ok(rows
            .into_iter()
            .map(|(id, n, s)| (id.into(), n, s))
            .collect())
    }

    /// Adds an alias unless it equals the canonical name or an existing alias (any case).
    /// Returns whether it was added.
    pub async fn add_alias(conn: &mut PgConnection, id: IngredientId, alias: &str) -> Result<bool> {
        let alias = alias.trim();
        if alias.is_empty() {
            return Ok(false);
        }
        let n = sqlx::query(
            "UPDATE ingredient SET aliases = array_append(aliases, $2)
             WHERE id = $1 AND lower(canonical_name) <> lower($2)
               AND NOT EXISTS (SELECT 1 FROM unnest(aliases) a WHERE lower(a) = lower($2))",
        )
        .bind(id.as_uuid())
        .bind(alias)
        .execute(conn)
        .await?
        .rows_affected();
        Ok(n > 0)
    }

    /// Writes the editable columns (not `origin`).
    pub async fn update(
        conn: &mut PgConnection,
        id: IngredientId,
        n: &NewIngredient,
    ) -> Result<IngredientRow> {
        let sql = format!(
            "UPDATE ingredient SET canonical_name = $2, aliases = $3, is_staple = $4, aisle_id = $5,
                    default_store_id = $6, default_unit = $7
             WHERE id = $1 RETURNING {ING_COLS}"
        );
        let row = sqlx::query_as::<_, RawIngredient>(&sql)
            .bind(id.as_uuid())
            .bind(&n.canonical_name)
            .bind(&n.aliases)
            .bind(n.is_staple)
            .bind(n.aisle_id.map(|a| a.as_uuid()))
            .bind(n.default_store_id.map(|a| a.as_uuid()))
            .bind(n.default_unit.as_ref().map(enum_str))
            .fetch_optional(conn)
            .await?
            .ok_or(StoreError::NotFound)?;
        row.try_into()
    }

    /// The staple flag is instant.
    pub async fn set_staple(conn: &mut PgConnection, id: IngredientId, on: bool) -> Result<()> {
        let n = sqlx::query("UPDATE ingredient SET is_staple = $2 WHERE id = $1")
            .bind(id.as_uuid())
            .bind(on)
            .execute(conn)
            .await?
            .rows_affected();
        if n == 0 {
            Err(StoreError::NotFound)
        } else {
            Ok(())
        }
    }

    /// Lines and items lose the link (SET NULL).
    pub async fn delete(conn: &mut PgConnection, id: IngredientId) -> Result<()> {
        let n = sqlx::query("DELETE FROM ingredient WHERE id = $1")
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

    /// Re-points lines and items from `from` to `into`, adds `from`'s name and aliases to
    /// `into`'s aliases (deduplicated, never its own name), then deletes `from`.
    pub async fn merge(
        conn: &mut PgConnection,
        from: IngredientId,
        into: IngredientId,
    ) -> Result<()> {
        if from == into {
            return Err(StoreError::Corrupt("merge into itself".into()));
        }
        let src = Self::get(conn, from).await?.ok_or(StoreError::NotFound)?;
        let dst = Self::get(conn, into).await?.ok_or(StoreError::NotFound)?;
        sqlx::query("UPDATE recipe_ingredient SET ingredient_id = $2 WHERE ingredient_id = $1")
            .bind(from.as_uuid())
            .bind(into.as_uuid())
            .execute(&mut *conn)
            .await?;
        sqlx::query("UPDATE grocery_item SET ingredient_id = $2 WHERE ingredient_id = $1")
            .bind(from.as_uuid())
            .bind(into.as_uuid())
            .execute(&mut *conn)
            .await?;
        let mut aliases = dst.aliases.clone();
        for name in std::iter::once(src.canonical_name).chain(src.aliases) {
            let lower = name.to_lowercase();
            if lower != dst.canonical_name.to_lowercase()
                && !aliases.iter().any(|a| a.to_lowercase() == lower)
            {
                aliases.push(name);
            }
        }
        sqlx::query("UPDATE ingredient SET aliases = $2 WHERE id = $1")
            .bind(into.as_uuid())
            .bind(&aliases)
            .execute(&mut *conn)
            .await?;
        sqlx::query("DELETE FROM ingredient WHERE id = $1")
            .bind(from.as_uuid())
            .execute(conn)
            .await?;
        Ok(())
    }

    pub async fn count(conn: &mut PgConnection) -> Result<i64> {
        Ok(sqlx::query_scalar("SELECT count(*) FROM ingredient")
            .fetch_one(conn)
            .await?)
    }
}
