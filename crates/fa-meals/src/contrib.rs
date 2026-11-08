//! What a meal plan entry contributes to the grocery list, and how a change to the plan moves
//! the items that came from it (arch/Meals.md §4.5, §4.6). The planner is pure; the loaders
//! read through the repos.

use std::collections::{HashMap, HashSet};

use fa_domain::{GroceryImpact, GroceryItemId, IngredientId, MealPlanEntryId};
use fa_store::{
    GroceryItemRow, GrocerySourceRow, IngredientRepo, IngredientRow, MealEntryRow, RecipeLineRepo,
    RecipeRepo,
};
use sqlx::PgConnection;

use crate::error::Result;
use crate::normalise::to_base;

const EPS: f64 = 1e-9;

/// Up to two quantities of one item, each in its own base unit (arch: volume and weight of one
/// ingredient stay two quantities on one item).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Amounts(pub Vec<(String, f64)>);

impl Amounts {
    pub fn from_slots(
        qty: Option<f64>,
        unit: Option<&str>,
        qty2: Option<f64>,
        unit2: Option<&str>,
    ) -> Self {
        let mut a = Amounts::default();
        for (q, u) in [(qty, unit), (qty2, unit2)] {
            if let Some(q) = q {
                a.0.push((u.unwrap_or("count").to_string(), q));
            }
        }
        a
    }

    pub fn from_item(i: &GroceryItemRow) -> Self {
        Self::from_slots(i.qty, i.unit.as_deref(), i.qty2, i.unit2.as_deref())
    }

    pub fn from_source(s: &GrocerySourceRow) -> Self {
        Self::from_slots(s.qty, s.unit.as_deref(), s.qty2, s.unit2.as_deref())
    }

    #[allow(clippy::type_complexity)]
    pub fn to_slots(&self) -> (Option<f64>, Option<String>, Option<f64>, Option<String>) {
        let a = self.0.first();
        let b = self.0.get(1);
        (
            a.map(|x| x.1),
            a.map(|x| x.0.clone()),
            b.map(|x| x.1),
            b.map(|x| x.0.clone()),
        )
    }

    pub fn get(&self, unit: &str) -> f64 {
        self.0.iter().find(|(u, _)| u == unit).map_or(0.0, |x| x.1)
    }

    /// Adds into the slot of that unit, or a free slot. False when both slots hold other units.
    pub fn add(&mut self, unit: &str, q: f64) -> bool {
        if let Some(x) = self.0.iter_mut().find(|(u, _)| u == unit) {
            x.1 += q;
            return true;
        }
        if self.0.len() < 2 {
            self.0.push((unit.to_string(), q));
            return true;
        }
        false
    }

    pub fn sub(&mut self, unit: &str, q: f64) {
        if let Some(i) = self.0.iter().position(|(u, _)| u == unit) {
            self.0[i].1 -= q;
            if self.0[i].1 <= EPS {
                self.0.remove(i);
            }
        }
    }

    /// Whether every amount of `other` fits (same unit, or a free slot).
    pub fn can_add(&self, other: &Amounts) -> bool {
        let mut c = self.clone();
        other.0.iter().all(|(u, q)| c.add(u, *q))
    }
}

/// What an item is identified by: its ingredient, or its cleaned text when unmatched.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ItemKey {
    Ingredient(IngredientId),
    Text(String),
}

pub fn key_of_item(i: &GroceryItemRow) -> ItemKey {
    match i.ingredient_id {
        Some(id) => ItemKey::Ingredient(id),
        None => ItemKey::Text(i.text.trim().to_lowercase()),
    }
}

/// One line group of an entry: everything that maps to one item.
#[derive(Debug, Clone, PartialEq)]
pub struct KeyContrib {
    pub key: ItemKey,
    /// The item text: the ingredient's name, or the cleaned line.
    pub text: String,
    pub ingredient: Option<IngredientRow>,
    pub amounts: Amounts,
}

impl KeyContrib {
    pub fn staple(&self) -> bool {
        self.ingredient.as_ref().is_some_and(|i| i.is_staple)
    }
}

/// The contributions of an entry at `servings` (the entry's, or a new value). Free-text entries
/// and recipes that are gone contribute nothing.
pub async fn entry_contribs(
    conn: &mut PgConnection,
    entry: &MealEntryRow,
    servings: Option<i32>,
) -> Result<Vec<KeyContrib>> {
    let Some(recipe_id) = entry.recipe_id else {
        return Ok(Vec::new());
    };
    let Some(recipe) = RecipeRepo::get(conn, recipe_id).await? else {
        return Ok(Vec::new());
    };
    let scale = match (servings, recipe.fields.servings) {
        (Some(e), Some(r)) if r > 0 => f64::from(e) / f64::from(r),
        _ => 1.0,
    };
    let lines = RecipeLineRepo::for_recipe(conn, recipe_id).await?;
    let mut cache: HashMap<IngredientId, Option<IngredientRow>> = HashMap::new();
    let mut out: Vec<KeyContrib> = Vec::new();
    for l in lines {
        let (key, text, ingredient) = match l.ingredient_id {
            Some(id) => {
                if let std::collections::hash_map::Entry::Vacant(v) = cache.entry(id) {
                    v.insert(IngredientRepo::get(conn, id).await?);
                }
                match cache.get(&id).cloned().flatten() {
                    Some(i) => (ItemKey::Ingredient(id), i.canonical_name.clone(), Some(i)),
                    None => continue,
                }
            }
            None => {
                let text = l
                    .item
                    .clone()
                    .filter(|s| !s.trim().is_empty())
                    .unwrap_or_else(|| l.raw_text.clone())
                    .trim()
                    .to_lowercase();
                (ItemKey::Text(text.clone()), text, None)
            }
        };
        let base = l.qty.and_then(|q| to_base(q * scale, l.unit.as_deref()));
        let pos = match out.iter().position(|k| k.key == key) {
            Some(p) => p,
            None => {
                out.push(KeyContrib {
                    key,
                    text,
                    ingredient,
                    amounts: Amounts::default(),
                });
                out.len() - 1
            }
        };
        if let Some((q, u)) = base {
            // A third unit for one ingredient in one recipe is dropped.
            out[pos].amounts.add(&u, q);
        }
    }
    Ok(out)
}

/// A source with its item.
#[derive(Debug, Clone)]
pub struct SrcCtx {
    pub src: GrocerySourceRow,
    pub item: GroceryItemRow,
}

/// What a plan change does to one item.
#[derive(Debug, Clone, PartialEq)]
pub struct ItemPlan {
    pub item: GroceryItemRow,
    pub old: Amounts,
    pub new: Amounts,
    /// No sources are left after the change: the item goes.
    pub remove: bool,
    pub entries: Vec<EntryPlan>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EntryPlan {
    pub entry_id: MealPlanEntryId,
    pub old_src: GrocerySourceRow,
    /// `None`: the entry no longer contributes to the item (the source goes).
    pub new_src: Option<Amounts>,
    pub before: Amounts,
    pub after: Amounts,
}

fn close(a: &Amounts, b: &Amounts) -> bool {
    a.0.len() == b.0.len()
        && a.0
            .iter()
            .all(|(u, q)| b.0.iter().any(|(u2, q2)| u == u2 && (q - q2).abs() < 1e-6))
}

/// Moves each affected item by the difference between what the entry gave it and what it gives
/// now (`new`; an entry with no key for the item gives nothing). Manual, checked and archived
/// items are never planned. `all_sources` lists every entry that has a source on an item.
pub fn plan_changes(
    ctxs: &[SrcCtx],
    new: &HashMap<MealPlanEntryId, Vec<KeyContrib>>,
    all_sources: &HashMap<GroceryItemId, HashSet<MealPlanEntryId>>,
) -> Vec<ItemPlan> {
    let mut order: Vec<GroceryItemId> = Vec::new();
    let mut by_item: HashMap<GroceryItemId, Vec<&SrcCtx>> = HashMap::new();
    for c in ctxs {
        if c.item.manual || c.item.checked_at.is_some() || c.item.archived_at.is_some() {
            continue;
        }
        if !by_item.contains_key(&c.item.id) {
            order.push(c.item.id);
        }
        by_item.entry(c.item.id).or_default().push(c);
    }
    let mut plans = Vec::new();
    for id in order {
        let group = &by_item[&id];
        let item = group[0].item.clone();
        let key = key_of_item(&item);
        let old = Amounts::from_item(&item);
        let mut running = old.clone();
        let mut gone: HashSet<MealPlanEntryId> = HashSet::new();
        let mut entries = Vec::new();
        for c in group {
            let before = running.clone();
            let old_src = Amounts::from_source(&c.src);
            let now = new
                .get(&c.src.entry_id)
                .and_then(|v| v.iter().find(|k| k.key == key))
                .map(|k| k.amounts.clone());
            for (u, q) in &old_src.0 {
                running.sub(u, *q);
            }
            if let Some(n) = &now {
                for (u, q) in &n.0 {
                    running.add(u, *q);
                }
            } else {
                gone.insert(c.src.entry_id);
            }
            entries.push(EntryPlan {
                entry_id: c.src.entry_id,
                old_src: c.src.clone(),
                new_src: now,
                before,
                after: running.clone(),
            });
        }
        let remaining = all_sources
            .get(&id)
            .map_or(0, |s| s.iter().filter(|e| !gone.contains(e)).count());
        plans.push(ItemPlan {
            item,
            old,
            new: running,
            remove: remaining == 0,
            entries,
        });
    }
    plans
}

impl ItemPlan {
    /// Whether applying this plan changes the item or any of its sources.
    pub fn changes_anything(&self) -> bool {
        self.remove
            || !close(&self.old, &self.new)
            || self.entries.iter().any(|e| match &e.new_src {
                None => true,
                Some(n) => !close(&Amounts::from_source(&e.old_src), n),
            })
    }
}

/// The impact rows (§4.6) of a set of item plans: one per (item, entry) that really changes.
pub fn impact_rows(plans: &[ItemPlan]) -> Vec<GroceryImpact> {
    let mut out = Vec::new();
    for p in plans {
        let last = p.entries.len().saturating_sub(1);
        for (i, e) in p.entries.iter().enumerate() {
            let old_src = Amounts::from_source(&e.old_src);
            let changed = match &e.new_src {
                None => true,
                Some(n) => !close(&old_src, n),
            };
            if !changed {
                continue;
            }
            let unit = old_src
                .0
                .first()
                .map(|x| x.0.clone())
                .or_else(|| p.old.0.first().map(|x| x.0.clone()));
            let get = |a: &Amounts| unit.as_deref().map(|u| a.get(u)).filter(|q| *q > EPS);
            out.push(GroceryImpact {
                item_id: p.item.id,
                entry_id: e.entry_id,
                text: p.item.text.clone(),
                from_qty: get(&e.before),
                to_qty: get(&e.after),
                unit,
                remove: p.remove && i == last,
            });
        }
    }
    out
}
