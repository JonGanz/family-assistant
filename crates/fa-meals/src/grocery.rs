//! The living grocery list: view, add, check, low, edit, delete, generate, reconcile and undo
//! (arch/Meals.md §4.5 to §4.7).

use std::collections::{HashMap, HashSet};

use chrono::NaiveDate;
use fa_domain::*;
use fa_store::{
    AisleRepo, GroceryItemEdit, GroceryItemRow, GroceryRepo, GrocerySourceRow, GroceryStoreRepo,
    MealEntryRow, MealPlanRepo, MealSlotRepo, MealsChangeRepo, MemberRepo, NewGroceryItem,
    StoreRow,
};
use sqlx::PgConnection;

use crate::change::{self, Inverse, ItemSnap, ItemUndo, SrcSnap};
use crate::contrib::{Amounts, ItemKey, entry_contribs, key_of_item, plan_changes};
use crate::error::{MealsError, Result};
use crate::matching::IngredientMatcher;
use crate::normalise::{Normaliser, display_pair, to_base};
use crate::plan::{contribs_after, load_ctxs};
use crate::service::{ActorRef, MealsService, publish};

const MAX_TEXT: usize = 200;
const MAX_GENERATE_DAYS: i64 = 366;

fn text_ok(t: &str) -> Result<String> {
    let t = t.trim();
    if t.is_empty() || t.chars().count() > MAX_TEXT {
        return Err(MealsError::invalid(
            "text",
            format!("1 to {MAX_TEXT} characters"),
        ));
    }
    Ok(t.to_string())
}

/// A quantity as typed, in base units when the unit is known, else as typed.
fn stored_qty(qty: f64, unit: Option<&str>) -> Result<(f64, String)> {
    if !qty.is_finite() || qty <= 0.0 || qty > 1_000_000.0 {
        return Err(MealsError::invalid("qty", "a positive number"));
    }
    Ok(match to_base(qty, unit) {
        Some(b) => b,
        None => (qty, unit.unwrap_or("count").trim().to_string()),
    })
}

/// Per store: aisle -> (position, hidden), with the household order for aisles it has no row for.
struct StoreOrders {
    household: HashMap<AisleId, i32>,
    stores: HashMap<GroceryStoreId, HashMap<AisleId, (i32, bool)>>,
}

impl StoreOrders {
    async fn load(conn: &mut PgConnection, stores: &[StoreRow]) -> Result<Self> {
        let household = AisleRepo::list(conn)
            .await?
            .into_iter()
            .map(|a| (a.id, a.ord))
            .collect();
        let mut map = HashMap::new();
        for s in stores {
            let rows = GroceryStoreRepo::aisles_for(conn, s.id).await?;
            map.insert(
                s.id,
                rows.into_iter()
                    .map(|r| (r.aisle_id, (r.ord, r.hidden)))
                    .collect(),
            );
        }
        Ok(Self {
            household,
            stores: map,
        })
    }

    /// The aisle an item is shown under, and its sort position. An aisle the store hides, or
    /// no aisle, is that store's "Other" (`None`, last).
    fn place(
        &self,
        store: Option<GroceryStoreId>,
        aisle: Option<AisleId>,
    ) -> (Option<AisleId>, i32) {
        let Some(a) = aisle else {
            return (None, i32::MAX);
        };
        match store.and_then(|s| self.stores.get(&s)) {
            Some(order) => match order.get(&a) {
                Some((_, true)) => (None, i32::MAX),
                Some((o, false)) => (Some(a), *o),
                None => (
                    Some(a),
                    self.household.get(&a).copied().unwrap_or(i32::MAX - 1),
                ),
            },
            None => (
                Some(a),
                self.household.get(&a).copied().unwrap_or(i32::MAX - 1),
            ),
        }
    }
}

async fn hints_of(
    conn: &mut PgConnection,
    items: &[GroceryItemId],
) -> Result<HashMap<GroceryItemId, Vec<SourceHint>>> {
    let mut out: HashMap<GroceryItemId, Vec<SourceHint>> = HashMap::new();
    for h in GroceryRepo::hints_for_items(conn, items).await? {
        out.entry(h.item_id).or_default().push(SourceHint {
            entry_id: h.entry_id,
            date: h.date,
            slot_name: h.slot_name,
        });
    }
    Ok(out)
}

fn view_of(
    r: &GroceryItemRow,
    aisle: Option<AisleId>,
    hints: Vec<SourceHint>,
    units: Units,
) -> GroceryItemView {
    GroceryItemView {
        id: r.id,
        text: r.text.clone(),
        qty: r.qty,
        unit: r.unit.clone(),
        qty2: r.qty2,
        unit2: r.unit2.clone(),
        display: display_pair(
            (r.qty, r.unit.as_deref()),
            (r.qty2, r.unit2.as_deref()),
            units,
        ),
        ingredient_id: r.ingredient_id,
        store_id: r.store_id,
        aisle_id: aisle,
        section: r.section,
        manual: r.manual,
        checked_at: r.checked_at,
        sources: hints,
        version: r.version,
    }
}

/// Views of some rows (any order kept), with hints and the store-effective aisle.
async fn views(
    conn: &mut PgConnection,
    rows: &[GroceryItemRow],
    units: Units,
) -> Result<Vec<GroceryItemView>> {
    let ids: Vec<GroceryItemId> = rows.iter().map(|r| r.id).collect();
    let mut hints = hints_of(conn, &ids).await?;
    let stores = GroceryStoreRepo::list(conn).await?;
    let orders = StoreOrders::load(conn, &stores).await?;
    Ok(rows
        .iter()
        .map(|r| {
            let (aisle, _) = orders.place(r.store_id, r.aisle_id);
            view_of(r, aisle, hints.remove(&r.id).unwrap_or_default(), units)
        })
        .collect())
}

fn view_one(mut v: Vec<GroceryItemView>) -> Result<GroceryItemView> {
    v.pop().ok_or(MealsError::NotFound)
}

fn slots4(a: &Amounts) -> (Option<f64>, Option<String>, Option<f64>, Option<String>) {
    a.to_slots()
}

struct Work {
    row: GroceryItemRow,
    amounts: Amounts,
    /// Snapshot before this run touched the item.
    before: Option<ItemSnap>,
}

fn source_row(
    item: GroceryItemId,
    entry: MealPlanEntryId,
    a: &Amounts,
    from: NaiveDate,
    to: NaiveDate,
) -> GrocerySourceRow {
    let (qty, unit, qty2, unit2) = slots4(a);
    GrocerySourceRow {
        item_id: item,
        entry_id: entry,
        qty,
        unit,
        qty2,
        unit2,
        generated_from: from,
        generated_to: to,
    }
}

impl MealsService {
    // ---- view ----

    /// Unarchived items (checked ones included) in list order: by store (in `grocery_store.ord`,
    /// "Any store" last), then by that store's aisle order. An item under an aisle its store
    /// hides, or with no aisle, shows `aisle_id = null` ("Other", last). Staples are separate.
    pub async fn grocery_view(&self, h: HouseholdId) -> Result<GroceryView> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let list = GroceryRepo::list_id(&mut tx, h).await?;
        let rows = GroceryRepo::items_open(&mut tx, list).await?;
        let stores = GroceryStoreRepo::list(&mut tx).await?;
        let aisles = AisleRepo::list(&mut tx).await?;
        let orders = StoreOrders::load(&mut tx, &stores).await?;
        let ids: Vec<GroceryItemId> = rows.iter().map(|r| r.id).collect();
        let mut hints = hints_of(&mut tx, &ids).await?;
        tx.commit().await?;

        let store_pos: HashMap<GroceryStoreId, usize> =
            stores.iter().enumerate().map(|(i, s)| (s.id, i)).collect();
        let mut listed: Vec<((usize, i32, String), GroceryItemView)> = Vec::new();
        let mut staples = Vec::new();
        for r in &rows {
            let (aisle, pos) = orders.place(r.store_id, r.aisle_id);
            let hs = hints.remove(&r.id).unwrap_or_default();
            if r.section == GrocerySection::Staples {
                if let Some(ingredient_id) = r.ingredient_id {
                    staples.push(StapleView {
                        item_id: r.id,
                        ingredient_id,
                        name: r.text.clone(),
                        needed_for: hs,
                    });
                }
                continue;
            }
            let store_key = r
                .store_id
                .and_then(|s| store_pos.get(&s).copied())
                .unwrap_or(usize::MAX);
            listed.push((
                (store_key, pos, r.text.to_lowercase()),
                view_of(r, aisle, hs, ctx.units),
            ));
        }
        listed.sort_by(|a, b| a.0.cmp(&b.0));
        staples.sort_by_key(|a| a.name.to_lowercase());
        Ok(GroceryView {
            stores: stores
                .into_iter()
                .map(|s| StoreView {
                    id: Some(s.id),
                    name: s.name,
                    ord: u32::try_from(s.ord).unwrap_or(0),
                })
                .collect(),
            aisles: aisles
                .into_iter()
                .map(|a| AisleView {
                    id: Some(a.id),
                    name: a.name,
                    ord: u32::try_from(a.ord).unwrap_or(0),
                })
                .collect(),
            items: listed.into_iter().map(|(_, v)| v).collect(),
            staples,
        })
    }

    // ---- add, check, low, edit, delete ----

    /// Adds a manual item. The text goes through the normaliser: a matched ingredient supplies
    /// its name, store and aisle; otherwise "Other", "Any store". A client `id` that already
    /// exists returns that item with `false` (D-118).
    pub async fn grocery_add(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        req: GroceryAddReq,
    ) -> Result<(GroceryItemView, bool)> {
        let text = text_ok(&req.text)?;
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        if let Some(id) = req.id
            && let Some(existing) = GroceryRepo::get_item(&mut tx, id).await?
        {
            let v = view_one(views(&mut tx, &[existing], ctx.units).await?)?;
            tx.commit().await?;
            return Ok((v, false));
        }
        if let Some(s) = req.store_id
            && GroceryStoreRepo::list(&mut tx)
                .await?
                .iter()
                .all(|x| x.id != s)
        {
            return Err(MealsError::invalid("store_id", "unknown store"));
        }
        let (settings, n) = self.settings_and_normaliser(&mut tx, h).await?;
        let parsed = Normaliser::parse_line(&text);
        let line = n.normalise(&text);
        let mut matched = None;
        if let Some(item) = &line.item {
            let folded = n.singular(&crate::normalise::fold(&parsed.item));
            matched = IngredientMatcher::new(settings.trigram_threshold)
                .find(&mut tx, &[item.as_str(), folded.as_str()])
                .await?
                .map(|m| m.ingredient);
        }
        let (qty, unit) = match req.qty {
            Some(q) => (Some(q), req.unit.clone()),
            None => (line.qty, line.unit.clone()),
        };
        let amounts = match qty {
            Some(q) => Some(stored_qty(q, unit.as_deref())?),
            None => None,
        };
        let item_text = match &matched {
            Some(i) => i.canonical_name.clone(),
            None if req.qty.is_none() && line.qty.is_some() && !parsed.item.trim().is_empty() => {
                parsed.item.trim().to_string()
            }
            None => text.clone(),
        };
        let list = GroceryRepo::list_id(&mut tx, h).await?;
        let (row, created) = GroceryRepo::insert_item(
            &mut tx,
            h,
            list,
            &NewGroceryItem {
                id: req.id,
                ingredient_id: matched.as_ref().map(|i| i.id),
                text: item_text,
                qty: amounts.as_ref().map(|a| a.0),
                unit: amounts.as_ref().map(|a| a.1.clone()),
                qty2: None,
                unit2: None,
                store_id: req
                    .store_id
                    .or(matched.as_ref().and_then(|i| i.default_store_id)),
                aisle_id: matched.as_ref().and_then(|i| i.aisle_id),
                section: GrocerySection::List,
                manual: true,
                added_by: actor.member,
                added_via_device: actor.device,
            },
        )
        .await?;
        if created {
            publish(&mut tx, h, "grocery.updated").await?;
        }
        let v = view_one(views(&mut tx, &[row], ctx.units).await?)?;
        tx.commit().await?;
        Ok((v, created))
    }

    /// Checks an item off whatever its version (check-offs win). Idempotent.
    pub async fn grocery_check(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        id: GroceryItemId,
    ) -> Result<GroceryItemView> {
        self.set_checked(h, id, Some(actor)).await
    }

    pub async fn grocery_uncheck(
        &self,
        h: HouseholdId,
        id: GroceryItemId,
    ) -> Result<GroceryItemView> {
        self.set_checked(h, id, None).await
    }

    async fn set_checked(
        &self,
        h: HouseholdId,
        id: GroceryItemId,
        check: Option<ActorRef>,
    ) -> Result<GroceryItemView> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let before = GroceryRepo::get_item(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let row = match check {
            Some(a) => GroceryRepo::check(&mut tx, id, a.member).await?,
            None => GroceryRepo::uncheck(&mut tx, id).await?,
        };
        if row.version != before.version {
            publish(&mut tx, h, "grocery.updated").await?;
        }
        let v = view_one(views(&mut tx, &[row], ctx.units).await?)?;
        tx.commit().await?;
        Ok(v)
    }

    /// "We are low": a staples item moves onto the list. A list item is returned unchanged.
    pub async fn grocery_low(&self, h: HouseholdId, id: GroceryItemId) -> Result<GroceryItemView> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let row = GroceryRepo::set_section(&mut tx, id, GrocerySection::List).await?;
        publish(&mut tx, h, "grocery.updated").await?;
        let v = view_one(views(&mut tx, &[row], ctx.units).await?)?;
        tx.commit().await?;
        Ok(v)
    }

    /// An adult edit with `If-Match`. Absent fields keep their value.
    pub async fn grocery_update(
        &self,
        h: HouseholdId,
        id: GroceryItemId,
        patch: GroceryItemPatch,
        if_match: i32,
    ) -> Result<GroceryItemView> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let cur = GroceryRepo::get_item(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let mut edit = GroceryItemEdit::default();
        if let Some(t) = &patch.text {
            edit.text = Some(text_ok(t)?);
        }
        match (patch.qty, patch.unit.as_deref()) {
            (Some(q), u) => {
                let (q, u) = stored_qty(q, u.or(cur.unit.as_deref()))?;
                edit.qty = Some(Some(q));
                edit.unit = Some(Some(u));
            }
            (None, Some(u)) => {
                edit.unit = Some(Some(u.trim().to_string()));
            }
            (None, None) => {}
        }
        if let Some(s) = patch.store_id {
            if GroceryStoreRepo::list(&mut tx)
                .await?
                .iter()
                .all(|x| x.id != s)
            {
                return Err(MealsError::invalid("store_id", "unknown store"));
            }
            edit.store_id = Some(Some(s));
        }
        GroceryRepo::update_item(&mut tx, id, &edit, if_match).await?;
        let row = GroceryRepo::get_item(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        publish(&mut tx, h, "grocery.updated").await?;
        let v = view_one(views(&mut tx, &[row], ctx.units).await?)?;
        tx.commit().await?;
        Ok(v)
    }

    pub async fn grocery_delete(&self, h: HouseholdId, id: GroceryItemId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        GroceryRepo::delete_item(&mut tx, id).await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- generate ----

    /// Builds the list from the picked entries or from a date range (visible slots only).
    /// Staple ingredients go to the staples section; the rest merges into an unchecked,
    /// generated list item with the same ingredient (or text), else a new item. An entry that
    /// already contributed to an item adds nothing. One undoable change.
    pub async fn generate(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        req: GenerateReq,
    ) -> Result<GenerateResult> {
        let mut tx = self.store.tx_for(h).await?;
        let (entries, from, to) = match (&req.entry_ids, req.from, req.to) {
            (Some(ids), None, None) => {
                let ids: Vec<MealPlanEntryId> = {
                    let mut seen = HashSet::new();
                    ids.iter().copied().filter(|i| seen.insert(*i)).collect()
                };
                if ids.is_empty() {
                    return Err(MealsError::invalid("entry_ids", "pick at least one entry"));
                }
                let rows = MealPlanRepo::many(&mut tx, &ids).await?;
                if rows.len() != ids.len() {
                    return Err(MealsError::NotFound);
                }
                let from = rows
                    .iter()
                    .map(|r| r.date)
                    .min()
                    .ok_or(MealsError::NotFound)?;
                let to = rows
                    .iter()
                    .map(|r| r.date)
                    .max()
                    .ok_or(MealsError::NotFound)?;
                (rows, from, to)
            }
            (None, Some(from), Some(to)) => {
                if to < from {
                    return Err(MealsError::invalid("to", "before from"));
                }
                if (to - from).num_days() + 1 > MAX_GENERATE_DAYS {
                    return Err(MealsError::invalid("to", "at most 366 days"));
                }
                let visible: HashSet<MealSlotId> = MealSlotRepo::list(&mut tx)
                    .await?
                    .into_iter()
                    .filter(|s| s.visible)
                    .map(|s| s.id)
                    .collect();
                let rows: Vec<MealEntryRow> = MealPlanRepo::range(&mut tx, from, to)
                    .await?
                    .into_iter()
                    .filter(|e| visible.contains(&e.slot_id))
                    .collect();
                (rows, from, to)
            }
            _ => {
                return Err(MealsError::invalid(
                    "from",
                    "send entry_ids, or from and to",
                ));
            }
        };

        let list = GroceryRepo::list_id(&mut tx, h).await?;
        let mut work: Vec<Work> = GroceryRepo::items_open(&mut tx, list)
            .await?
            .into_iter()
            .map(|row| Work {
                amounts: Amounts::from_item(&row),
                row,
                before: None,
            })
            .collect();
        let entry_ids: Vec<MealPlanEntryId> = entries.iter().map(|e| e.id).collect();
        let mut has_source: HashSet<(MealPlanEntryId, ItemKey)> = HashSet::new();
        for s in GroceryRepo::sources_for_entries(&mut tx, &entry_ids).await? {
            let item = match work.iter().find(|w| w.row.id == s.item_id) {
                Some(w) => Some(w.row.clone()),
                None => GroceryRepo::get_item(&mut tx, s.item_id).await?,
            };
            if let Some(i) = item {
                has_source.insert((s.entry_id, key_of_item(&i)));
            }
        }

        let (mut added, mut merged, mut staples) = (0u32, 0u32, 0u32);
        let mut created_items = Vec::new();
        let mut added_sources = Vec::new();
        for e in &entries {
            for kc in entry_contribs(&mut tx, e, e.servings).await? {
                if has_source.contains(&(e.id, kc.key.clone())) {
                    continue;
                }
                let staple = kc.staple();
                let target = work.iter().position(|w| {
                    w.row.archived_at.is_none()
                        && !w.row.manual
                        && key_of_item(&w.row) == kc.key
                        && if staple {
                            w.row.section == GrocerySection::Staples
                        } else {
                            w.row.section == GrocerySection::List && w.row.checked_at.is_none()
                        }
                        && w.amounts.can_add(&kc.amounts)
                });
                let item_id = match target {
                    Some(i) => {
                        let w = &mut work[i];
                        if w.before.is_none() {
                            w.before = Some(ItemSnap::from(&w.row));
                        }
                        for (u, q) in &kc.amounts.0 {
                            w.amounts.add(u, *q);
                        }
                        let (q, u, q2, u2) = slots4(&w.amounts);
                        GroceryRepo::set_quantities(
                            &mut tx,
                            w.row.id,
                            q,
                            u.as_deref(),
                            q2,
                            u2.as_deref(),
                        )
                        .await?;
                        (w.row.qty, w.row.unit, w.row.qty2, w.row.unit2) = (q, u, q2, u2);
                        if staple {
                            staples += 1;
                        } else {
                            merged += 1;
                        }
                        w.row.id
                    }
                    None => {
                        let (q, u, q2, u2) = slots4(&kc.amounts);
                        let ing = kc.ingredient.as_ref();
                        let (row, _) = GroceryRepo::insert_item(
                            &mut tx,
                            h,
                            list,
                            &NewGroceryItem {
                                id: None,
                                ingredient_id: ing.map(|i| i.id),
                                text: kc.text.clone(),
                                qty: q,
                                unit: u,
                                qty2: q2,
                                unit2: u2,
                                store_id: ing.and_then(|i| i.default_store_id),
                                aisle_id: ing.and_then(|i| i.aisle_id),
                                section: if staple {
                                    GrocerySection::Staples
                                } else {
                                    GrocerySection::List
                                },
                                manual: false,
                                added_by: actor.member,
                                added_via_device: actor.device,
                            },
                        )
                        .await?;
                        created_items.push(row.id);
                        if staple {
                            staples += 1;
                        } else {
                            added += 1;
                        }
                        let id = row.id;
                        work.push(Work {
                            amounts: kc.amounts.clone(),
                            row,
                            before: None,
                        });
                        id
                    }
                };
                GroceryRepo::upsert_source(
                    &mut tx,
                    h,
                    &source_row(item_id, e.id, &kc.amounts, from, to),
                )
                .await?;
                added_sources.push((item_id, e.id));
                has_source.insert((e.id, kc.key));
            }
        }
        let change = change::record(
            &mut tx,
            h,
            actor,
            MealsChangeKind::Generate,
            &Inverse::Generate {
                created_items,
                restore_items: work.iter().filter_map(|w| w.before.clone()).collect(),
                added_sources,
            },
            self.now(),
        )
        .await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(GenerateResult {
            change_id: change.id,
            added,
            merged,
            staples,
        })
    }

    // ---- reconcile ----

    /// Recomputes the given entries' contributions from the current plan: a deleted entry gives
    /// nothing, a moved or re-served one gives what it gives now. Manual and checked items are
    /// never touched. Items left with no source are removed. One undoable change.
    pub async fn reconcile(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        req: ReconcileReq,
    ) -> Result<ReconcileResult> {
        let ids: Vec<MealPlanEntryId> = {
            let mut seen = HashSet::new();
            req.entry_ids
                .iter()
                .copied()
                .filter(|i| seen.insert(*i))
                .collect()
        };
        if ids.is_empty() {
            return Err(MealsError::invalid("entry_ids", "pick at least one entry"));
        }
        let mut tx = self.store.tx_for(h).await?;
        let existing: HashMap<MealPlanEntryId, MealEntryRow> = MealPlanRepo::many(&mut tx, &ids)
            .await?
            .into_iter()
            .map(|e| (e.id, e))
            .collect();
        let mut sources = GroceryRepo::sources_for_entries(&mut tx, &ids).await?;
        let gone: Vec<MealPlanEntryId> = ids
            .iter()
            .copied()
            .filter(|i| !existing.contains_key(i))
            .collect();
        let parked = if gone.is_empty() {
            Vec::new()
        } else {
            change::parked_for(&mut tx, &gone, self.now()).await?
        };
        for (_, _, snaps) in &parked {
            sources.extend(snaps.iter().cloned().map(GrocerySourceRow::from));
        }
        let mut new = HashMap::new();
        for id in &ids {
            let contribs = match existing.get(id) {
                Some(e) => {
                    let own: Vec<GrocerySourceRow> = sources
                        .iter()
                        .filter(|s| s.entry_id == *id)
                        .cloned()
                        .collect();
                    if own.is_empty() {
                        Vec::new()
                    } else {
                        contribs_after(&mut tx, e, &own).await?
                    }
                }
                None => Vec::new(),
            };
            new.insert(*id, contribs);
        }
        let (ctxs, all) = load_ctxs(&mut tx, sources).await?;
        let plans: Vec<_> = plan_changes(&ctxs, &new, &all)
            .into_iter()
            .filter(|p| p.changes_anything())
            .collect();
        let mut undo = Vec::new();
        for p in &plans {
            undo.push(ItemUndo {
                item: ItemSnap::from(&p.item),
                removed: p.remove,
                sources: p
                    .entries
                    .iter()
                    .map(|e| SrcSnap::from(&e.old_src))
                    .collect(),
            });
            if p.remove {
                GroceryRepo::delete_item(&mut tx, p.item.id).await?;
                continue;
            }
            let (q, u, q2, u2) = slots4(&p.new);
            GroceryRepo::set_quantities(&mut tx, p.item.id, q, u.as_deref(), q2, u2.as_deref())
                .await?;
            for e in &p.entries {
                match &e.new_src {
                    Some(a) if existing.contains_key(&e.entry_id) => {
                        GroceryRepo::upsert_source(
                            &mut tx,
                            h,
                            &source_row(
                                p.item.id,
                                e.entry_id,
                                a,
                                e.old_src.generated_from,
                                e.old_src.generated_to,
                            ),
                        )
                        .await?;
                    }
                    _ => GroceryRepo::delete_source(&mut tx, p.item.id, e.entry_id).await?,
                }
            }
        }
        for (change_id, _, _) in parked {
            MealsChangeRepo::mark_undone(&mut tx, change_id).await?;
        }
        let change = change::record(
            &mut tx,
            h,
            actor,
            MealsChangeKind::Reconcile,
            &Inverse::Reconcile { items: undo },
            self.now(),
        )
        .await?;
        publish(&mut tx, h, "grocery.updated").await?;
        tx.commit().await?;
        Ok(ReconcileResult {
            change_id: change.id,
            changed: u32::try_from(plans.len()).unwrap_or(u32::MAX),
        })
    }

    // ---- undo ----

    /// Undoes a generate, reconcile, template apply, swap or move within ten minutes. Allowed
    /// for the member or device that made it, or any adult member.
    pub async fn undo(&self, h: HouseholdId, actor: ActorRef, id: MealsChangeId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        let ch = MealsChangeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let inverse = change::from_json(&ch.inverse)?;
        if inverse.is_pending() {
            return Err(MealsError::NotFound);
        }
        let same = (actor.member.is_some() && actor.member == ch.actor_member)
            || (actor.device.is_some() && actor.device == ch.actor_device);
        let adult = match actor.member {
            Some(m) => MemberRepo::get(&mut tx, m)
                .await?
                .is_some_and(|m| m.value.role == Role::Adult),
            None => false,
        };
        if !same && !adult {
            return Err(MealsError::NotFound);
        }
        if ch.undone_at.is_some() {
            return Err(MealsError::AlreadyUndone);
        }
        if self.now() > ch.expires_at {
            return Err(MealsError::Expired);
        }
        if !MealsChangeRepo::mark_undone(&mut tx, id).await? {
            return Err(MealsError::AlreadyUndone);
        }
        let (plan, grocery) = apply_inverse(&mut tx, h, inverse).await?;
        if plan {
            publish(&mut tx, h, "mealplan.updated").await?;
        }
        if grocery {
            publish(&mut tx, h, "grocery.updated").await?;
        }
        tx.commit().await?;
        Ok(())
    }
}

/// Applies an inverse. Returns which of the plan and the list changed.
async fn apply_inverse(
    conn: &mut PgConnection,
    h: HouseholdId,
    inverse: Inverse,
) -> Result<(bool, bool)> {
    match inverse {
        Inverse::Swap { a, b } => {
            if MealPlanRepo::get(conn, a).await?.is_some()
                && MealPlanRepo::get(conn, b).await?.is_some()
            {
                MealPlanRepo::swap(conn, a, b).await?;
            }
            Ok((true, false))
        }
        Inverse::Move {
            entry,
            date,
            slot_id,
            ord,
        } => {
            sqlx::query(
                "UPDATE meal_plan_entry SET date = $2, slot_id = $3, ord = $4, version = version + 1
                 WHERE id = $1",
            )
            .bind(entry.as_uuid())
            .bind(date)
            .bind(slot_id.as_uuid())
            .bind(ord)
            .execute(&mut *conn)
            .await?;
            Ok((true, false))
        }
        Inverse::TemplateApply { deleted, created } => {
            for id in created {
                if MealPlanRepo::get(conn, id).await?.is_some() {
                    MealPlanRepo::delete(conn, id).await?;
                }
            }
            for s in &deleted {
                change::restore_entry(conn, h, s).await?;
            }
            Ok((true, false))
        }
        Inverse::Generate {
            created_items,
            restore_items,
            added_sources,
        } => {
            for (item, entry) in added_sources {
                GroceryRepo::delete_source(conn, item, entry).await?;
            }
            for s in restore_items {
                if GroceryRepo::get_item(conn, s.id).await?.is_some() {
                    GroceryRepo::set_quantities(
                        conn,
                        s.id,
                        s.qty,
                        s.unit.as_deref(),
                        s.qty2,
                        s.unit2.as_deref(),
                    )
                    .await?;
                }
            }
            for id in created_items {
                // An item checked off in the meantime stays.
                if let Some(i) = GroceryRepo::get_item(conn, id).await?
                    && i.checked_at.is_none()
                {
                    GroceryRepo::delete_item(conn, id).await?;
                }
            }
            Ok((false, true))
        }
        Inverse::Reconcile { items } => {
            for u in items {
                let s = &u.item;
                if u.removed {
                    let list = GroceryRepo::list_id(conn, h).await?;
                    GroceryRepo::insert_item(
                        conn,
                        h,
                        list,
                        &NewGroceryItem {
                            id: Some(s.id),
                            ingredient_id: s.ingredient_id,
                            text: s.text.clone(),
                            qty: s.qty,
                            unit: s.unit.clone(),
                            qty2: s.qty2,
                            unit2: s.unit2.clone(),
                            store_id: s.store_id,
                            aisle_id: s.aisle_id,
                            section: s.section,
                            manual: s.manual,
                            added_by: s.added_by,
                            added_via_device: s.added_via_device,
                        },
                    )
                    .await?;
                } else if GroceryRepo::get_item(conn, s.id).await?.is_some() {
                    GroceryRepo::set_quantities(
                        conn,
                        s.id,
                        s.qty,
                        s.unit.as_deref(),
                        s.qty2,
                        s.unit2.as_deref(),
                    )
                    .await?;
                } else {
                    continue;
                }
                for src in u.sources {
                    if MealPlanRepo::get(conn, src.entry_id).await?.is_some() {
                        GroceryRepo::upsert_source(conn, h, &src.into()).await?;
                    }
                }
            }
            Ok((false, true))
        }
        Inverse::Pending { .. } => Err(MealsError::NotFound),
    }
}
