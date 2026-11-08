//! The meal plan: range, add, update, delete, swap and move (arch/Meals.md §4.3), each with the
//! grocery impact of §4.6.

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use fa_domain::*;
use fa_store::{
    EntryContent, EntryPatch, GroceryRepo, GrocerySourceRow, MealEntryRow, MealPlanRepo,
    MealSlotRepo, NewEntry, RecipeRepo,
};
use sqlx::PgConnection;

use crate::change::{self, Inverse};
use crate::contrib::{KeyContrib, SrcCtx, entry_contribs, impact_rows, plan_changes};
use crate::error::{MealsError, Result};
use crate::service::{ActorRef, HouseholdCtx, MealsService, opt_u32, publish, to_i32};

const MAX_RANGE_DAYS: i64 = 62;
const MAX_FREE_TEXT: usize = 200;

pub(crate) fn entry_view(r: MealEntryRow) -> MealPlanEntryView {
    MealPlanEntryView {
        id: r.id,
        date: r.date,
        slot_id: r.slot_id,
        ord: u32::try_from(r.ord).unwrap_or(0),
        recipe_id: r.recipe_id,
        title: r.title,
        free_text: r.free_text,
        servings: opt_u32(r.servings),
        version: r.version,
    }
}

fn check_servings(v: Option<u32>) -> Result<Option<i32>> {
    match v {
        Some(n) if !(1..=100).contains(&n) => Err(MealsError::invalid("servings", "1 to 100")),
        other => Ok(other.map(to_i32)),
    }
}

/// Exactly one of recipe and free text, as an entry content.
pub(crate) fn content_of(
    recipe: Option<RecipeId>,
    free_text: Option<&str>,
) -> Result<EntryContent> {
    let text = free_text.map(str::trim).filter(|t| !t.is_empty());
    match (recipe, text) {
        (Some(r), None) => Ok(EntryContent::Recipe(r)),
        (None, Some(t)) => {
            if t.chars().count() > MAX_FREE_TEXT {
                return Err(MealsError::invalid(
                    "free_text",
                    format!("at most {MAX_FREE_TEXT} characters"),
                ));
            }
            Ok(EntryContent::FreeText(t.to_string()))
        }
        _ => Err(MealsError::invalid(
            "recipe_id",
            "send either recipe_id or free_text",
        )),
    }
}

/// The recipe's servings, for an entry that has none of its own.
pub(crate) async fn default_servings(
    conn: &mut PgConnection,
    content: &EntryContent,
    servings: Option<i32>,
) -> Result<Option<i32>> {
    match content {
        EntryContent::Recipe(r) => {
            let recipe = RecipeRepo::get(conn, *r)
                .await?
                .ok_or_else(|| MealsError::invalid("recipe_id", "unknown recipe"))?;
            Ok(servings.or(recipe.fields.servings))
        }
        EntryContent::FreeText(_) => Ok(servings),
    }
}

pub(crate) async fn slot_exists(conn: &mut PgConnection, slot: MealSlotId) -> Result<()> {
    MealSlotRepo::get(conn, slot)
        .await?
        .map(|_| ())
        .ok_or_else(|| MealsError::invalid("slot_id", "unknown slot"))
}

impl MealsService {
    /// Writes are allowed from today (household time zone) to the horizon end.
    pub(crate) fn check_writable(&self, ctx: &HouseholdCtx, date: NaiveDate) -> Result<()> {
        if date < self.today(ctx) {
            return Err(MealsError::invalid("date", "earlier dates are read-only"));
        }
        if date > self.horizon_end(ctx) {
            return Err(MealsError::invalid(
                "date",
                "later than the planning horizon",
            ));
        }
        Ok(())
    }

    pub async fn plan_range(
        &self,
        h: HouseholdId,
        from: NaiveDate,
        to: NaiveDate,
    ) -> Result<Vec<MealPlanEntryView>> {
        if to < from {
            return Err(MealsError::invalid("to", "before from"));
        }
        if (to - from).num_days() + 1 > MAX_RANGE_DAYS {
            return Err(MealsError::invalid(
                "to",
                format!("at most {MAX_RANGE_DAYS} days"),
            ));
        }
        let mut tx = self.store.tx_for(h).await?;
        let rows = MealPlanRepo::range(&mut tx, from, to).await?;
        tx.rollback().await?;
        Ok(rows.into_iter().map(entry_view).collect())
    }

    /// Today's entries in visible slots, for the dashboard.
    pub async fn dashboard_meals(&self, h: HouseholdId) -> Result<Vec<DashboardMeal>> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let rows = MealPlanRepo::day_meals(&mut tx, self.today(&ctx)).await?;
        tx.rollback().await?;
        Ok(rows
            .into_iter()
            .map(|r| DashboardMeal {
                slot_name: r.slot_name,
                time: format_slot_time(r.slot_time),
                title: r.title,
                recipe_id: r.recipe_id,
            })
            .collect())
    }

    /// Appends an entry to its slot (`ord` = max + 1).
    pub async fn plan_add(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        input: MealPlanEntryInput,
    ) -> Result<PlanChangeResult> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        self.check_writable(&ctx, input.date)?;
        slot_exists(&mut tx, input.slot_id).await?;
        let content = content_of(input.recipe_id, input.free_text.as_deref())?;
        let servings = default_servings(&mut tx, &content, check_servings(input.servings)?).await?;
        let row = MealPlanRepo::insert(
            &mut tx,
            h,
            &NewEntry {
                date: input.date,
                slot_id: input.slot_id,
                content,
                servings,
                created_by: actor.member,
            },
        )
        .await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(PlanChangeResult {
            entries: vec![entry_view(row)],
            change_id: None,
            grocery_impact: Vec::new(),
        })
    }

    /// Edits an entry. A new date or slot appends it there and is one undoable `move`.
    pub async fn plan_update(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        id: MealPlanEntryId,
        patch: MealPlanEntryPatch,
        if_match: i32,
    ) -> Result<PlanChangeResult> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let cur = MealPlanRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        self.check_writable(&ctx, cur.date)?;
        let date = patch.date.unwrap_or(cur.date);
        let slot = patch.slot_id.unwrap_or(cur.slot_id);
        if date != cur.date {
            self.check_writable(&ctx, date)?;
        }
        if slot != cur.slot_id {
            slot_exists(&mut tx, slot).await?;
        }
        let content = if patch.recipe_id.is_some() || patch.free_text.is_some() {
            Some(content_of(patch.recipe_id, patch.free_text.as_deref())?)
        } else {
            None
        };
        let servings = check_servings(patch.servings)?;
        if let Some(EntryContent::Recipe(r)) = &content
            && RecipeRepo::get(&mut tx, *r).await?.is_none()
        {
            return Err(MealsError::invalid("recipe_id", "unknown recipe"));
        }
        // The entry as it will be, to work out what the list should do.
        let mut after = cur.clone();
        after.date = date;
        after.slot_id = slot;
        if let Some(c) = &content {
            match c {
                EntryContent::Recipe(r) => {
                    after.recipe_id = Some(*r);
                    after.free_text = None;
                }
                EntryContent::FreeText(t) => {
                    after.recipe_id = None;
                    after.free_text = Some(t.clone());
                }
            }
        }
        if servings.is_some() {
            after.servings = servings;
        }
        let sources = GroceryRepo::sources_for_entries(&mut tx, &[id]).await?;
        let grocery_impact = self.impact_of_after(&mut tx, &after, &sources).await?;
        let moved = date != cur.date || slot != cur.slot_id;
        MealPlanRepo::update(
            &mut tx,
            id,
            &EntryPatch {
                date: patch.date,
                slot_id: patch.slot_id,
                content,
                servings: servings.map(Some),
            },
            if_match,
        )
        .await?;
        let change_id = if moved {
            let c = change::record(
                &mut tx,
                h,
                actor,
                MealsChangeKind::Move,
                &Inverse::Move {
                    entry: id,
                    date: cur.date,
                    slot_id: cur.slot_id,
                    ord: cur.ord,
                },
                self.now(),
            )
            .await?;
            Some(c.id)
        } else {
            None
        };
        publish(&mut tx, h, "mealplan.updated").await?;
        let row = MealPlanRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        tx.commit().await?;
        Ok(PlanChangeResult {
            entries: vec![entry_view(row)],
            change_id,
            grocery_impact,
        })
    }

    pub async fn plan_delete(
        &self,
        h: HouseholdId,
        _actor: ActorRef,
        id: MealPlanEntryId,
    ) -> Result<PlanChangeResult> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let cur = MealPlanRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        self.check_writable(&ctx, cur.date)?;
        let grocery_impact = self.impact_of_removal(&mut tx, h, &[id]).await?;
        MealPlanRepo::delete(&mut tx, id).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(PlanChangeResult {
            entries: Vec::new(),
            change_id: None,
            grocery_impact,
        })
    }

    /// Swaps `a` with `b`, or moves `a` to the end of `date` + `slot_id`. One undoable change.
    pub async fn plan_swap(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        req: SwapReq,
    ) -> Result<PlanChangeResult> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let a = MealPlanRepo::get(&mut tx, req.a)
            .await?
            .ok_or(MealsError::NotFound)?;
        self.check_writable(&ctx, a.date)?;
        let (kind, inverse, afters, touched): (
            MealsChangeKind,
            Inverse,
            Vec<MealEntryRow>,
            Vec<MealPlanEntryId>,
        ) = match (req.b, req.date, req.slot_id) {
            (Some(bid), None, None) => {
                if bid == req.a {
                    return Err(MealsError::invalid("b", "cannot swap an entry with itself"));
                }
                let b = MealPlanRepo::get(&mut tx, bid)
                    .await?
                    .ok_or(MealsError::NotFound)?;
                self.check_writable(&ctx, b.date)?;
                let mut na = a.clone();
                (na.date, na.slot_id, na.ord) = (b.date, b.slot_id, b.ord);
                let mut nb = b.clone();
                (nb.date, nb.slot_id, nb.ord) = (a.date, a.slot_id, a.ord);
                (
                    MealsChangeKind::Swap,
                    Inverse::Swap { a: req.a, b: bid },
                    vec![na, nb],
                    vec![req.a, bid],
                )
            }
            (None, Some(date), Some(slot)) => {
                self.check_writable(&ctx, date)?;
                slot_exists(&mut tx, slot).await?;
                let mut na = a.clone();
                na.date = date;
                na.slot_id = slot;
                (
                    MealsChangeKind::Move,
                    Inverse::Move {
                        entry: req.a,
                        date: a.date,
                        slot_id: a.slot_id,
                        ord: a.ord,
                    },
                    vec![na],
                    vec![req.a],
                )
            }
            _ => {
                return Err(MealsError::invalid(
                    "b",
                    "send b, or date and slot_id, to say where a goes",
                ));
            }
        };
        let sources = GroceryRepo::sources_for_entries(&mut tx, &touched).await?;
        let mut grocery_impact = Vec::new();
        for after in &afters {
            let own: Vec<GrocerySourceRow> = sources
                .iter()
                .filter(|s| s.entry_id == after.id)
                .cloned()
                .collect();
            grocery_impact.extend(self.impact_of_after(&mut tx, after, &own).await?);
        }
        match (req.b, req.date, req.slot_id) {
            (Some(bid), _, _) => MealPlanRepo::swap(&mut tx, req.a, bid).await?,
            (None, Some(date), Some(slot)) => {
                let v = a.version;
                MealPlanRepo::update(
                    &mut tx,
                    req.a,
                    &EntryPatch {
                        date: Some(date),
                        slot_id: Some(slot),
                        ..Default::default()
                    },
                    v,
                )
                .await?;
            }
            _ => {}
        }
        let c = change::record(&mut tx, h, actor, kind, &inverse, self.now()).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        let rows = MealPlanRepo::many(&mut tx, &touched).await?;
        tx.commit().await?;
        Ok(PlanChangeResult {
            entries: rows.into_iter().map(entry_view).collect(),
            change_id: Some(c.id),
            grocery_impact,
        })
    }

    // ---- grocery impact (§4.6) ----

    /// What the list should do when an entry becomes `after` (a new date, recipe or servings).
    /// An entry that leaves the range it was generated for contributes nothing.
    pub(crate) async fn impact_of_after(
        &self,
        conn: &mut PgConnection,
        after: &MealEntryRow,
        sources: &[GrocerySourceRow],
    ) -> Result<Vec<GroceryImpact>> {
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        let contribs = contribs_after(conn, after, sources).await?;
        let mut new = HashMap::new();
        new.insert(after.id, contribs);
        impact(conn, &new).await
    }

    /// The impact of deleting entries, and parks their contributions for `reconcile` (the
    /// sources cascade away with the entries).
    pub(crate) async fn impact_of_removal(
        &self,
        conn: &mut PgConnection,
        h: HouseholdId,
        ids: &[MealPlanEntryId],
    ) -> Result<Vec<GroceryImpact>> {
        let sources = GroceryRepo::sources_for_entries(conn, ids).await?;
        if sources.is_empty() {
            return Ok(Vec::new());
        }
        let new: HashMap<MealPlanEntryId, Vec<KeyContrib>> =
            ids.iter().map(|i| (*i, Vec::new())).collect();
        let rows = impact(conn, &new).await?;
        if !rows.is_empty() {
            change::park(conn, h, &sources, self.now()).await?;
        }
        Ok(rows)
    }
}

/// The contributions of `after`, or none when its date is outside every range it was generated for.
pub(crate) async fn contribs_after(
    conn: &mut PgConnection,
    after: &MealEntryRow,
    sources: &[GrocerySourceRow],
) -> Result<Vec<KeyContrib>> {
    let inside = sources
        .iter()
        .any(|s| s.generated_from <= after.date && after.date <= s.generated_to);
    if !inside {
        return Ok(Vec::new());
    }
    entry_contribs(conn, after, after.servings).await
}

/// Loads the items behind the entries' sources and plans what `new` does to them.
pub(crate) async fn load_ctxs(
    conn: &mut PgConnection,
    sources: Vec<GrocerySourceRow>,
) -> Result<(
    Vec<SrcCtx>,
    HashMap<GroceryItemId, HashSet<MealPlanEntryId>>,
)> {
    let mut item_ids: Vec<GroceryItemId> = Vec::new();
    for s in &sources {
        if !item_ids.contains(&s.item_id) {
            item_ids.push(s.item_id);
        }
    }
    let mut ctxs = Vec::new();
    for s in sources {
        if let Some(item) = GroceryRepo::get_item(conn, s.item_id).await? {
            ctxs.push(SrcCtx { src: s, item });
        }
    }
    let mut all: HashMap<GroceryItemId, HashSet<MealPlanEntryId>> = HashMap::new();
    for h in GroceryRepo::hints_for_items(conn, &item_ids).await? {
        all.entry(h.item_id).or_default().insert(h.entry_id);
    }
    Ok((ctxs, all))
}

/// The §4.6 rows for these new contributions.
pub(crate) async fn impact(
    conn: &mut PgConnection,
    new: &HashMap<MealPlanEntryId, Vec<KeyContrib>>,
) -> Result<Vec<GroceryImpact>> {
    let ids: Vec<MealPlanEntryId> = new.keys().copied().collect();
    let sources = GroceryRepo::sources_for_entries(conn, &ids).await?;
    let (ctxs, all) = load_ctxs(conn, sources).await?;
    Ok(impact_rows(&plan_changes(&ctxs, new, &all)))
}
