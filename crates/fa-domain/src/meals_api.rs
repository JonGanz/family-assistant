//! M5 request and response DTOs (plan/M5.md, Domain types).
//!
//! Wire formats: dates are `YYYY-MM-DD`, slot times are local `"HH:MM"`, timestamps are RFC 3339,
//! quantities are `f64`. Nullable fields are `field?: T | null` in TypeScript.

use chrono::{DateTime, NaiveDate, NaiveTime, Utc};
use serde::{Deserialize, Serialize};
use ts_rs::TS;
use utoipa::ToSchema;

use crate::entities::*;
use crate::ids::*;
use crate::timefmt;

macro_rules! dto {
    ($(#[$m:meta])* $v:vis struct $name:ident { $($body:tt)* }) => {
        $(#[$m])*
        #[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS, ToSchema)]
        #[ts(export, export_to = "../../../apps/web/src/gen/types/")]
        $v struct $name { $($body)* }
    };
}

/// Why a slot time is not valid.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("a slot time is \"HH:MM\", 00:00 to 23:59")]
pub struct SlotTimeError;

/// Parses a slot time. Only `"HH:MM"` (two digits each, 00:00 to 23:59) is accepted.
pub fn parse_slot_time(s: &str) -> Result<NaiveTime, SlotTimeError> {
    let b = s.as_bytes();
    let ok = b.len() == 5 && b[2] == b':' && [0, 1, 3, 4].iter().all(|&i| b[i].is_ascii_digit());
    if !ok {
        return Err(SlotTimeError);
    }
    NaiveTime::parse_from_str(s, "%H:%M").map_err(|_| SlotTimeError)
}

/// The wire form of a slot time, `"HH:MM"`.
pub fn format_slot_time(t: NaiveTime) -> String {
    timefmt::format_hhmm(t)
}

// ---- recipes ----
dto!(
    pub struct RecipeSummary {
        pub id: RecipeId,
        pub title: String,
        pub has_image: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub total_minutes: Option<u32>,
        pub tags: Vec<String>,
        pub favourite: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub rating_avg: Option<f32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub my_rating: Option<u8>,
        #[serde(default)]
        #[ts(as = "Option<String>", optional = nullable)]
        #[schema(value_type = Option<String>)]
        pub last_planned: Option<NaiveDate>,
    }
);

dto!(
    /// One page of `GET /recipes`; `next_cursor` is opaque.
    pub struct RecipePage {
        pub items: Vec<RecipeSummary>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub next_cursor: Option<String>,
    }
);

dto!(
    pub struct RecipeView {
        pub id: RecipeId,
        pub title: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub source_url: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub yield_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub prep_minutes: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub cook_minutes: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub total_minutes: Option<u32>,
        pub import_method: ImportMethod,
        pub tags: Vec<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub note: Option<String>,
        pub favourite: bool,
        pub has_image: bool,
        pub lines: Vec<RecipeLineView>,
        pub steps: Vec<String>,
        pub ratings: RatingsView,
        pub version: i32,
    }
);

dto!(
    pub struct RecipeLineView {
        pub id: RecipeLineId,
        pub ord: u32,
        pub raw_text: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub item: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub note: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_id: Option<IngredientId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_name: Option<String>,
        pub match_state: MatchState,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub proposal: Option<IngredientProposal>,
    }
);

dto!(
    /// An LLM proposal for an unmatched line. An adult accepts it or picks another match.
    pub struct IngredientProposal {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_id: Option<IngredientId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub new_name: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub aisle: Option<String>,
        pub confidence: f32,
    }
);

dto!(
    /// Create and update body. `lines` are the raw ingredient lines; `steps` are the step texts.
    pub struct RecipeInput {
        pub title: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub source_url: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub yield_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub prep_minutes: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub cook_minutes: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub total_minutes: Option<u32>,
        pub import_method: ImportMethod,
        pub tags: Vec<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub note: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub image_url: Option<String>,
        pub lines: Vec<String>,
        pub steps: Vec<String>,
    }
);

dto!(
    pub struct MemberRating {
        pub member_id: MemberId,
        pub stars: u8,
    }
);

dto!(
    pub struct RatingsView {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub avg: Option<f32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub mine: Option<u8>,
        pub by_member: Vec<MemberRating>,
    }
);

dto!(
    /// `url` or `text`.
    pub struct RecipeImportReq {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub url: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub text: Option<String>,
    }
);

dto!(
    /// Pick an existing ingredient, or create `new_name` (in `aisle_id`).
    pub struct LineMatchReq {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_id: Option<IngredientId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub new_name: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub aisle_id: Option<AisleId>,
    }
);

// ---- ingredients ----
dto!(
    pub struct IngredientView {
        pub id: IngredientId,
        pub canonical_name: String,
        pub aliases: Vec<String>,
        pub is_staple: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub aisle_id: Option<AisleId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub default_store_id: Option<GroceryStoreId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub default_unit: Option<BaseUnit>,
        pub origin: IngredientOrigin,
    }
);

dto!(
    pub struct IngredientInput {
        pub canonical_name: String,
        pub aliases: Vec<String>,
        pub is_staple: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub aisle_id: Option<AisleId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub default_store_id: Option<GroceryStoreId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub default_unit: Option<BaseUnit>,
    }
);

dto!(
    pub struct IngredientMergeReq {
        pub into: IngredientId,
    }
);

// ---- settings, slots, aisles, stores ----
dto!(
    pub struct MealsSettingsView {
        pub grocery_archive_hours: u32,
        pub ingredient_llm: bool,
        pub trigram_threshold: f32,
        pub descriptors: Vec<String>,
        pub meal_plan_horizon_weeks: u32,
    }
);

dto!(
    /// Every field optional; absent keeps the stored value.
    pub struct MealsSettingsPatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub grocery_archive_hours: Option<u32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_llm: Option<bool>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub trigram_threshold: Option<f32>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub descriptors: Option<Vec<String>>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub meal_plan_horizon_weeks: Option<u32>,
    }
);

dto!(
    /// `time` is local `"HH:MM"`. `id` absent on a new slot in `PUT /meals/slots`.
    pub struct MealSlotView {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub id: Option<MealSlotId>,
        pub name: String,
        pub time: String,
        pub visible: bool,
        pub ord: u32,
    }
);

dto!(
    pub struct AisleView {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub id: Option<AisleId>,
        pub name: String,
        pub ord: u32,
    }
);

dto!(
    pub struct StoreView {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub id: Option<GroceryStoreId>,
        pub name: String,
        pub ord: u32,
    }
);

dto!(
    /// One aisle in a store's own order (D-121).
    pub struct StoreAisleView {
        pub aisle_id: AisleId,
        pub ord: u32,
        pub hidden: bool,
    }
);

// ---- meal plan ----
dto!(
    /// `date` is `YYYY-MM-DD`. `title` is the recipe title, or the free text.
    pub struct MealPlanEntryView {
        pub id: MealPlanEntryId,
        #[ts(type = "string")]
        pub date: NaiveDate,
        pub slot_id: MealSlotId,
        pub ord: u32,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
        pub title: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub free_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
        pub version: i32,
    }
);

dto!(
    /// Either `recipe_id` or `free_text`. The entry is appended to the slot.
    pub struct MealPlanEntryInput {
        #[ts(type = "string")]
        pub date: NaiveDate,
        pub slot_id: MealSlotId,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub free_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
    }
);

dto!(
    pub struct MealPlanEntryPatch {
        #[serde(default)]
        #[ts(as = "Option<String>", optional = nullable)]
        #[schema(value_type = Option<String>)]
        pub date: Option<NaiveDate>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub slot_id: Option<MealSlotId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub free_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
    }
);

dto!(
    /// Swap `a` with `b`, or move `a` into the (empty) slot `date` + `slot_id`.
    pub struct SwapReq {
        pub a: MealPlanEntryId,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub b: Option<MealPlanEntryId>,
        #[serde(default)]
        #[ts(as = "Option<String>", optional = nullable)]
        #[schema(value_type = Option<String>)]
        pub date: Option<NaiveDate>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub slot_id: Option<MealSlotId>,
    }
);

dto!(
    /// A generated grocery item that a plan change touches.
    pub struct GroceryImpact {
        pub item_id: GroceryItemId,
        pub entry_id: MealPlanEntryId,
        pub text: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub from_qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub to_qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit: Option<String>,
        pub remove: bool,
    }
);

dto!(
    pub struct PlanChangeResult {
        pub entries: Vec<MealPlanEntryView>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub change_id: Option<MealsChangeId>,
        pub grocery_impact: Vec<GroceryImpact>,
    }
);

// ---- templates ----
dto!(
    pub struct MealTemplateSummary {
        pub id: MealTemplateId,
        pub name: String,
        pub days: u32,
        pub slot_ids: Vec<MealSlotId>,
        pub version: i32,
    }
);

dto!(
    pub struct TemplateCellView {
        pub day: u32,
        pub slot_id: MealSlotId,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
        pub title: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub free_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
    }
);

dto!(
    pub struct MealTemplateView {
        pub id: MealTemplateId,
        pub name: String,
        pub days: u32,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub note: Option<String>,
        pub cells: Vec<TemplateCellView>,
        pub version: i32,
    }
);

dto!(
    pub struct TemplateCellInput {
        pub day: u32,
        pub slot_id: MealSlotId,
        pub ord: u32,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub free_text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub servings: Option<u32>,
    }
);

dto!(
    pub struct MealTemplateInput {
        pub name: String,
        pub days: u32,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub note: Option<String>,
        pub cells: Vec<TemplateCellInput>,
    }
);

dto!(
    /// Save a template from a plan range; `from` and `to` are `YYYY-MM-DD`, at most 366 days.
    pub struct TemplateFromPlanReq {
        pub name: String,
        #[ts(type = "string")]
        pub from: NaiveDate,
        #[ts(type = "string")]
        pub to: NaiveDate,
        pub slot_ids: Vec<MealSlotId>,
    }
);

dto!(
    pub struct TemplateDuplicateReq {
        pub name: String,
    }
);

dto!(
    pub struct TemplatePreviewReq {
        #[ts(type = "string")]
        pub start: NaiveDate,
    }
);

dto!(
    pub struct SlotRef {
        #[ts(type = "string")]
        pub date: NaiveDate,
        pub slot_id: MealSlotId,
    }
);

dto!(
    /// Slots in `add_slots` keep their entries and get the items appended; every other target slot is replaced.
    pub struct TemplateApplyReq {
        #[ts(type = "string")]
        pub start: NaiveDate,
        pub add_slots: Vec<SlotRef>,
    }
);

dto!(
    pub struct TemplateTarget {
        pub day: u32,
        #[ts(type = "string")]
        pub date: NaiveDate,
        pub slot_id: MealSlotId,
        pub slot_visible: bool,
        pub items: Vec<TemplateCellView>,
        pub existing: Vec<MealPlanEntryView>,
    }
);

dto!(
    pub struct TemplatePreview {
        pub targets: Vec<TemplateTarget>,
    }
);

// ---- grocery ----
dto!(
    /// One place an item came from, for the "for Tue dinner" hint.
    pub struct SourceHint {
        pub entry_id: MealPlanEntryId,
        #[ts(type = "string")]
        pub date: NaiveDate,
        pub slot_name: String,
    }
);

dto!(
    /// `display` is the quantity text in household units, e.g. `"2 ½ cups"`, `"750 g"`, `"3"`.
    pub struct GroceryItemView {
        pub id: GroceryItemId,
        pub text: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub qty2: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit2: Option<String>,
        pub display: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub ingredient_id: Option<IngredientId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub store_id: Option<GroceryStoreId>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub aisle_id: Option<AisleId>,
        pub section: GrocerySection,
        pub manual: bool,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub checked_at: Option<DateTime<Utc>>,
        pub sources: Vec<SourceHint>,
        pub version: i32,
    }
);

dto!(
    pub struct StapleView {
        pub item_id: GroceryItemId,
        pub ingredient_id: IngredientId,
        pub name: String,
        pub needed_for: Vec<SourceHint>,
    }
);

dto!(
    pub struct GroceryView {
        pub stores: Vec<StoreView>,
        pub aisles: Vec<AisleView>,
        pub items: Vec<GroceryItemView>,
        pub staples: Vec<StapleView>,
    }
);

dto!(
    /// `id` may be client-generated so a replayed offline add is idempotent (D-118).
    pub struct GroceryAddReq {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub id: Option<GroceryItemId>,
        pub text: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub store_id: Option<GroceryStoreId>,
    }
);

dto!(
    pub struct GroceryItemPatch {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub text: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub qty: Option<f64>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub unit: Option<String>,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub store_id: Option<GroceryStoreId>,
    }
);

dto!(
    /// A range (`from`..`to`, `YYYY-MM-DD`) or picked entries.
    pub struct GenerateReq {
        #[serde(default)]
        #[ts(optional = nullable)]
        pub entry_ids: Option<Vec<MealPlanEntryId>>,
        #[serde(default)]
        #[ts(as = "Option<String>", optional = nullable)]
        #[schema(value_type = Option<String>)]
        pub from: Option<NaiveDate>,
        #[serde(default)]
        #[ts(as = "Option<String>", optional = nullable)]
        #[schema(value_type = Option<String>)]
        pub to: Option<NaiveDate>,
    }
);

dto!(
    pub struct GenerateResult {
        pub change_id: MealsChangeId,
        pub added: u32,
        pub merged: u32,
        pub staples: u32,
    }
);

dto!(
    pub struct ReconcileReq {
        pub entry_ids: Vec<MealPlanEntryId>,
    }
);

dto!(
    pub struct ReconcileResult {
        pub change_id: MealsChangeId,
        pub changed: u32,
    }
);

// ---- dashboard ----
dto!(
    /// One planned meal on the dashboard; `time` is local `"HH:MM"`.
    pub struct DashboardMeal {
        pub slot_name: String,
        pub time: String,
        pub title: String,
        #[serde(default)]
        #[ts(optional = nullable)]
        pub recipe_id: Option<RecipeId>,
    }
);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slot_time_accepts_hhmm_only() {
        let t = parse_slot_time("18:00").unwrap();
        assert_eq!(format_slot_time(t), "18:00");
        assert_eq!(
            parse_slot_time("00:00").unwrap(),
            NaiveTime::from_hms_opt(0, 0, 0).unwrap()
        );
        assert_eq!(format_slot_time(parse_slot_time("23:59").unwrap()), "23:59");
        for bad in [
            "", "7:30", "24:00", "12:60", "18:00:00", "1800", "ab:cd", "18:0", " 18:00", "+8:00",
            "18-00",
        ] {
            assert!(parse_slot_time(bad).is_err(), "{bad:?}");
        }
    }

    #[test]
    fn dto_wire_shapes() {
        let v: GroceryAddReq = serde_json::from_str(r#"{"text":"milk"}"#).unwrap();
        assert!(v.id.is_none() && v.store_id.is_none());
        let p: MealPlanEntryPatch = serde_json::from_str(r#"{"date":"2026-11-09"}"#).unwrap();
        assert_eq!(p.date, NaiveDate::from_ymd_opt(2026, 11, 9));
        let s = serde_json::to_value(MealSlotView {
            id: None,
            name: "Dinner".into(),
            time: "18:00".into(),
            visible: true,
            ord: 2,
        })
        .unwrap();
        assert_eq!(s["time"], "18:00");
    }
}
