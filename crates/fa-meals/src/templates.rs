//! Meal templates: made in the editor, saved from a plan range or duplicated; previewed and
//! applied with a Replace or Add choice per slot (arch/Meals.md §4.4).

use std::collections::{HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use fa_domain::*;
use fa_store::{
    EntryContent, MealEntryRow, MealPlanRepo, MealSlotRepo, MealTemplateRepo, MealTemplateRow,
    NewEntry, NewTemplate, NewTemplateCell, TemplateCellRow,
};
use sqlx::PgConnection;

use crate::change::{self, EntrySnap, Inverse};
use crate::error::{MealsError, Result};
use crate::plan::{content_of, default_servings, entry_view, slot_exists};
use crate::service::{ActorRef, MealsService, opt_u32, publish, to_i32, to_u32};

const MAX_DAYS: u32 = 366;

fn cell_view(c: TemplateCellRow) -> TemplateCellView {
    TemplateCellView {
        day: to_u32(c.day),
        slot_id: c.slot_id,
        recipe_id: c.recipe_id,
        title: c.title,
        free_text: c.free_text,
        servings: opt_u32(c.servings),
    }
}

fn template_view(t: MealTemplateRow, cells: Vec<TemplateCellRow>) -> MealTemplateView {
    MealTemplateView {
        id: t.id,
        name: t.name,
        days: to_u32(t.days),
        note: t.note,
        cells: cells.into_iter().map(cell_view).collect(),
        version: t.version,
    }
}

fn check_name(n: &str) -> Result<String> {
    let n = n.trim();
    if n.is_empty() || n.chars().count() > 100 {
        return Err(MealsError::invalid("name", "1 to 100 characters"));
    }
    Ok(n.to_string())
}

/// Validates the header and cells of an input and numbers the cells per (day, slot).
async fn checked(
    conn: &mut PgConnection,
    i: &MealTemplateInput,
) -> Result<(NewTemplate, Vec<NewTemplateCell>)> {
    if !(1..=MAX_DAYS).contains(&i.days) {
        return Err(MealsError::invalid("days", "1 to 366"));
    }
    let note = i
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if note.as_ref().is_some_and(|n| n.chars().count() > 2000) {
        return Err(MealsError::invalid("note", "at most 2000 characters"));
    }
    let slots: HashSet<MealSlotId> = MealSlotRepo::list(conn)
        .await?
        .into_iter()
        .map(|s| s.id)
        .collect();
    let mut sorted: Vec<&TemplateCellInput> = i.cells.iter().collect();
    sorted.sort_by_key(|c| (c.day, c.slot_id, c.ord));
    let mut next: HashMap<(u32, MealSlotId), i32> = HashMap::new();
    let mut cells = Vec::with_capacity(sorted.len());
    for c in sorted {
        if c.day < 1 || c.day > i.days {
            return Err(MealsError::invalid(
                "cells",
                "a cell is outside the template's days",
            ));
        }
        if !slots.contains(&c.slot_id) {
            return Err(MealsError::invalid("slot_id", "unknown slot"));
        }
        let content = content_of(c.recipe_id, c.free_text.as_deref())?;
        if let EntryContent::Recipe(r) = &content
            && fa_store::RecipeRepo::get(conn, *r).await?.is_none()
        {
            return Err(MealsError::invalid("recipe_id", "unknown recipe"));
        }
        let servings = match c.servings {
            Some(n) if !(1..=100).contains(&n) => {
                return Err(MealsError::invalid("servings", "1 to 100"));
            }
            other => other.map(to_i32),
        };
        let ord = next.entry((c.day, c.slot_id)).or_insert(0);
        cells.push(NewTemplateCell {
            day: to_i32(c.day),
            slot_id: c.slot_id,
            ord: *ord,
            content,
            servings,
        });
        *ord += 1;
    }
    Ok((
        NewTemplate {
            name: check_name(&i.name)?,
            days: to_i32(i.days),
            note,
        },
        cells,
    ))
}

/// The cells grouped by (day, slot) in order: the apply targets.
fn targets_of(cells: &[TemplateCellRow]) -> Vec<((i32, MealSlotId), Vec<&TemplateCellRow>)> {
    let mut out: Vec<((i32, MealSlotId), Vec<&TemplateCellRow>)> = Vec::new();
    for c in cells {
        match out.iter_mut().find(|(k, _)| *k == (c.day, c.slot_id)) {
            Some((_, v)) => v.push(c),
            None => out.push(((c.day, c.slot_id), vec![c])),
        }
    }
    out
}

fn date_of(start: NaiveDate, day: i32) -> NaiveDate {
    start + Duration::days(i64::from(day - 1))
}

impl MealsService {
    pub async fn template_list(&self, h: HouseholdId) -> Result<Vec<MealTemplateSummary>> {
        let mut tx = self.store.tx_for(h).await?;
        let rows = MealTemplateRepo::list(&mut tx).await?;
        tx.rollback().await?;
        Ok(rows
            .into_iter()
            .map(|r| MealTemplateSummary {
                id: r.row.id,
                name: r.row.name,
                days: to_u32(r.row.days),
                slot_ids: r.slot_ids,
                version: r.row.version,
            })
            .collect())
    }

    pub async fn template_get(
        &self,
        h: HouseholdId,
        id: MealTemplateId,
    ) -> Result<MealTemplateView> {
        let mut tx = self.store.tx_for(h).await?;
        let (t, cells) = MealTemplateRepo::get_with_cells(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        tx.rollback().await?;
        Ok(template_view(t, cells))
    }

    pub async fn template_create(
        &self,
        h: HouseholdId,
        input: MealTemplateInput,
    ) -> Result<MealTemplateView> {
        let mut tx = self.store.tx_for(h).await?;
        let (t, cells) = checked(&mut tx, &input).await?;
        let row = MealTemplateRepo::insert(&mut tx, h, &t, &cells).await?;
        let cells = MealTemplateRepo::cells(&mut tx, row.id).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(template_view(row, cells))
    }

    /// Replaces the header and the cells whole. Entries applied earlier are not touched.
    pub async fn template_update(
        &self,
        h: HouseholdId,
        id: MealTemplateId,
        input: MealTemplateInput,
        if_match: i32,
    ) -> Result<MealTemplateView> {
        let mut tx = self.store.tx_for(h).await?;
        let (t, cells) = checked(&mut tx, &input).await?;
        MealTemplateRepo::update_with_cells(&mut tx, id, &t, &cells, if_match).await?;
        let (row, cells) = MealTemplateRepo::get_with_cells(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(template_view(row, cells))
    }

    pub async fn template_delete(&self, h: HouseholdId, id: MealTemplateId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        MealTemplateRepo::delete(&mut tx, id).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// Saves every entry of the given slots in `from..=to` as cells (day = date - from + 1,
    /// same order). The copies are detached from the plan.
    pub async fn template_from_plan(
        &self,
        h: HouseholdId,
        req: TemplateFromPlanReq,
    ) -> Result<MealTemplateView> {
        let name = check_name(&req.name)?;
        if req.to < req.from {
            return Err(MealsError::invalid("to", "before from"));
        }
        let days = (req.to - req.from).num_days() + 1;
        if days > i64::from(MAX_DAYS) {
            return Err(MealsError::invalid("to", "at most 366 days"));
        }
        let mut tx = self.store.tx_for(h).await?;
        for s in &req.slot_ids {
            slot_exists(&mut tx, *s).await?;
        }
        let entries = MealPlanRepo::range(&mut tx, req.from, req.to).await?;
        let mut cells = Vec::new();
        for e in entries.iter().filter(|e| req.slot_ids.contains(&e.slot_id)) {
            let content = content_of(e.recipe_id, e.free_text.as_deref())?;
            cells.push(NewTemplateCell {
                day: i32::try_from((e.date - req.from).num_days() + 1).unwrap_or(1),
                slot_id: e.slot_id,
                ord: e.ord,
                content,
                servings: e.servings,
            });
        }
        let row = MealTemplateRepo::insert(
            &mut tx,
            h,
            &NewTemplate {
                name,
                days: i32::try_from(days).unwrap_or(1),
                note: None,
            },
            &cells,
        )
        .await?;
        let cells = MealTemplateRepo::cells(&mut tx, row.id).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(template_view(row, cells))
    }

    pub async fn template_duplicate(
        &self,
        h: HouseholdId,
        id: MealTemplateId,
        req: TemplateDuplicateReq,
    ) -> Result<MealTemplateView> {
        let name = check_name(&req.name)?;
        let mut tx = self.store.tx_for(h).await?;
        let row = MealTemplateRepo::duplicate(&mut tx, id, &name).await?;
        let cells = MealTemplateRepo::cells(&mut tx, row.id).await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(template_view(row, cells))
    }

    /// For every target (date, slot) the template's items and what is there now. Hidden slots
    /// are included and marked.
    pub async fn template_preview(
        &self,
        h: HouseholdId,
        id: MealTemplateId,
        req: TemplatePreviewReq,
    ) -> Result<TemplatePreview> {
        let mut tx = self.store.tx_for(h).await?;
        let (_, cells) = MealTemplateRepo::get_with_cells(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let visible: HashMap<MealSlotId, bool> = MealSlotRepo::list(&mut tx)
            .await?
            .into_iter()
            .map(|s| (s.id, s.visible))
            .collect();
        let mut targets = Vec::new();
        for ((day, slot), items) in targets_of(&cells) {
            let date = date_of(req.start, day);
            let existing = MealPlanRepo::entries_at(&mut tx, date, slot).await?;
            targets.push(TemplateTarget {
                day: to_u32(day),
                date,
                slot_id: slot,
                slot_visible: visible.get(&slot).copied().unwrap_or(true),
                items: items.into_iter().cloned().map(cell_view).collect(),
                existing: existing.into_iter().map(entry_view).collect(),
            });
        }
        tx.rollback().await?;
        Ok(TemplatePreview { targets })
    }

    /// Applies the template from `start` in one transaction. Slots in `add_slots` keep their
    /// entries and get the items appended; every other target slot is replaced. One undoable
    /// change; the grocery impact covers the replaced entries.
    pub async fn template_apply(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        id: MealTemplateId,
        req: TemplateApplyReq,
    ) -> Result<PlanChangeResult> {
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let (_, cells) = MealTemplateRepo::get_with_cells(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let targets = targets_of(&cells);
        for ((day, _), _) in &targets {
            self.check_writable(&ctx, date_of(req.start, *day))?;
        }
        let add: HashSet<(NaiveDate, MealSlotId)> =
            req.add_slots.iter().map(|s| (s.date, s.slot_id)).collect();

        let mut deleted: Vec<MealEntryRow> = Vec::new();
        for ((day, slot), _) in &targets {
            let date = date_of(req.start, *day);
            if !add.contains(&(date, *slot)) {
                deleted.extend(MealPlanRepo::entries_at(&mut tx, date, *slot).await?);
            }
        }
        let deleted_ids: Vec<MealPlanEntryId> = deleted.iter().map(|e| e.id).collect();
        let grocery_impact = self.impact_of_removal(&mut tx, h, &deleted_ids).await?;
        for ((day, slot), _) in &targets {
            let date = date_of(req.start, *day);
            if !add.contains(&(date, *slot)) {
                MealPlanRepo::delete_at(&mut tx, date, *slot).await?;
            }
        }
        let mut created = Vec::new();
        for ((day, slot), items) in &targets {
            let date = date_of(req.start, *day);
            for c in items {
                let content = content_of(c.recipe_id, c.free_text.as_deref())?;
                let servings = default_servings(&mut tx, &content, c.servings).await?;
                created.push(
                    MealPlanRepo::insert(
                        &mut tx,
                        h,
                        &NewEntry {
                            date,
                            slot_id: *slot,
                            content,
                            servings,
                            created_by: actor.member,
                        },
                    )
                    .await?,
                );
            }
        }
        let change = change::record(
            &mut tx,
            h,
            actor,
            MealsChangeKind::TemplateApply,
            &Inverse::TemplateApply {
                deleted: deleted.iter().map(EntrySnap::from).collect(),
                created: created.iter().map(|e| e.id).collect(),
            },
            self.now(),
        )
        .await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(PlanChangeResult {
            entries: created.into_iter().map(entry_view).collect(),
            change_id: Some(change.id),
            grocery_impact,
        })
    }
}
