//! The meal plan and templates over a real Postgres.

mod common;

use common::*;
use fa_domain::*;
use fa_meals::{ActorRef, MealsError};
use sqlx::PgPool;

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn entries_append_in_order_and_writes_are_bounded(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let r = e.recipe("Soup", Some(4), &["1 onion"]).await;
    let a = e.plan(d(10), dinner, &r, None).await;
    let b = e.free(d(10), dinner, "Takeaway").await;
    let c = e.plan(d(10), dinner, &r, Some(2)).await;
    assert_eq!((a.ord, b.ord, c.ord), (0, 1, 2));
    assert_eq!(a.servings, Some(4), "defaults to the recipe's");
    assert_eq!(c.servings, Some(2));
    assert_eq!(b.title, "Takeaway");
    let day = e.svc.plan_range(e.h, d(10), d(10)).await.unwrap();
    assert_eq!(
        day.iter().map(|x| x.id).collect::<Vec<_>>(),
        [a.id, b.id, c.id]
    );

    // Yesterday is history; beyond the horizon (2 weeks) is not plannable yet.
    let yesterday = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(8),
                slot_id: dinner,
                recipe_id: None,
                free_text: Some("x".into()),
                servings: None,
            },
        )
        .await;
    assert!(matches!(
        yesterday,
        Err(MealsError::Invalid { field: "date", .. })
    ));
    let far = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(24),
                slot_id: dinner,
                recipe_id: None,
                free_text: Some("x".into()),
                servings: None,
            },
        )
        .await;
    assert!(matches!(
        far,
        Err(MealsError::Invalid { field: "date", .. })
    ));
    e.svc
        .update_settings(
            e.h,
            MealsSettingsPatch {
                meal_plan_horizon_weeks: Some(4),
                ..sp()
            },
        )
        .await
        .unwrap();
    e.free(d(24), dinner, "Later").await;
    // Today counts, in the household's time zone (it is Monday morning in New York).
    e.free(d(9), dinner, "Today").await;
    // One of recipe or free text, and a real slot.
    let both = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(10),
                slot_id: dinner,
                recipe_id: Some(r.id),
                free_text: Some("x".into()),
                servings: None,
            },
        )
        .await;
    assert!(matches!(both, Err(MealsError::Invalid { .. })));
    let neither = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(10),
                slot_id: dinner,
                recipe_id: None,
                free_text: None,
                servings: None,
            },
        )
        .await;
    assert!(matches!(neither, Err(MealsError::Invalid { .. })));
    let slot = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(10),
                slot_id: MealSlotId::new(),
                recipe_id: None,
                free_text: Some("x".into()),
                servings: None,
            },
        )
        .await;
    assert!(matches!(
        slot,
        Err(MealsError::Invalid {
            field: "slot_id",
            ..
        })
    ));
    // Ranges are at most 62 days.
    assert!(e.svc.plan_range(e.h, d(1), d(30)).await.is_ok());
    assert!(matches!(
        e.svc
            .plan_range(
                e.h,
                d(1),
                chrono::NaiveDate::from_ymd_opt(2027, 2, 1).unwrap()
            )
            .await,
        Err(MealsError::Invalid { .. })
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn the_day_rolls_over_in_the_household_time_zone(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    // 03:00 UTC on the 10th is still the evening of the 9th in New York.
    e.at(chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 11, 10, 3, 0, 0).unwrap());
    e.free(d(9), dinner, "Still today").await;
    e.at(chrono::TimeZone::with_ymd_and_hms(&chrono::Utc, 2026, 11, 10, 6, 0, 0).unwrap());
    let r = e
        .svc
        .plan_add(
            e.h,
            e.actor,
            MealPlanEntryInput {
                date: d(9),
                slot_id: dinner,
                recipe_id: None,
                free_text: Some("x".into()),
                servings: None,
            },
        )
        .await;
    assert!(matches!(r, Err(MealsError::Invalid { field: "date", .. })));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn swap_and_move_are_undoable(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let lunch = e.slot("Lunch").await;
    let a = e.free(d(10), dinner, "Pizza").await;
    let b = e.free(d(11), dinner, "Curry").await;
    let c = e.free(d(11), dinner, "Salad").await;
    let res = e
        .svc
        .plan_swap(
            e.h,
            e.actor,
            SwapReq {
                a: a.id,
                b: Some(c.id),
                date: None,
                slot_id: None,
            },
        )
        .await
        .unwrap();
    let change = res.change_id.unwrap();
    let day10 = e.svc.plan_range(e.h, d(10), d(10)).await.unwrap();
    let day11 = e.svc.plan_range(e.h, d(11), d(11)).await.unwrap();
    assert_eq!(
        day10.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
        ["Salad"]
    );
    assert_eq!(
        day11.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
        ["Curry", "Pizza"]
    );
    e.svc.undo(e.h, e.actor, change).await.unwrap();
    let day10 = e.svc.plan_range(e.h, d(10), d(10)).await.unwrap();
    let day11 = e.svc.plan_range(e.h, d(11), d(11)).await.unwrap();
    assert_eq!(day10[0].title, "Pizza");
    assert_eq!(
        day11.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
        ["Curry", "Salad"]
    );

    // Move into an empty slot, then undo puts it back at its old place and order.
    let res = e
        .svc
        .plan_swap(
            e.h,
            e.actor,
            SwapReq {
                a: b.id,
                b: None,
                date: Some(d(12)),
                slot_id: Some(lunch),
            },
        )
        .await
        .unwrap();
    assert_eq!(res.entries[0].date, d(12));
    e.svc
        .undo(e.h, e.actor, res.change_id.unwrap())
        .await
        .unwrap();
    let day11 = e.svc.plan_range(e.h, d(11), d(11)).await.unwrap();
    assert_eq!(
        day11
            .iter()
            .map(|x| (x.title.as_str(), x.ord))
            .collect::<Vec<_>>(),
        [("Curry", 0), ("Salad", 1)]
    );
    // Needs b, or a place.
    assert!(matches!(
        e.svc
            .plan_swap(
                e.h,
                e.actor,
                SwapReq {
                    a: a.id,
                    b: None,
                    date: None,
                    slot_id: None
                }
            )
            .await,
        Err(MealsError::Invalid { .. })
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn update_checks_the_version_and_the_past(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let a = e.free(d(10), dinner, "Pizza").await;
    let upd = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            a.id,
            MealPlanEntryPatch {
                free_text: Some("Pizza night".into()),
                ..pp()
            },
            a.version,
        )
        .await
        .unwrap();
    assert_eq!(upd.entries[0].title, "Pizza night");
    assert!(upd.change_id.is_none());
    let stale = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            a.id,
            MealPlanEntryPatch {
                free_text: Some("x".into()),
                ..pp()
            },
            a.version,
        )
        .await;
    assert!(matches!(stale, Err(MealsError::Conflict(_))));
    // Today's entry may be edited; yesterday's may not.
    e.at(base_time() + chrono::Duration::days(2));
    let late = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            a.id,
            MealPlanEntryPatch {
                servings: Some(3),
                ..pp()
            },
            upd.entries[0].version,
        )
        .await;
    assert!(matches!(
        late,
        Err(MealsError::Invalid { field: "date", .. })
    ));
    assert!(matches!(
        e.svc.plan_delete(e.h, e.actor, a.id).await,
        Err(MealsError::Invalid { .. })
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn deleting_a_recipe_keeps_its_entries_as_text(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    e.plan(d(10), dinner, &r, None).await;
    e.svc.delete_recipe(e.h, r.id).await.unwrap();
    let day = e.svc.plan_range(e.h, d(10), d(10)).await.unwrap();
    assert_eq!(
        (
            day[0].title.as_str(),
            day[0].free_text.as_deref(),
            day[0].recipe_id
        ),
        ("Chili", Some("Chili"), None)
    );
    assert!(matches!(
        e.svc.get_recipe(e.h, None, r.id).await,
        Err(MealsError::NotFound)
    ));
}

async fn template_with_three_days(e: &Env, recipe: &RecipeView) -> MealTemplateView {
    let dinner = e.dinner().await;
    let lunch = e.slot("Lunch").await;
    let cell = |day, slot, ord, recipe: Option<RecipeId>, text: Option<&str>| TemplateCellInput {
        day,
        slot_id: slot,
        ord,
        recipe_id: recipe,
        free_text: text.map(str::to_string),
        servings: None,
    };
    e.svc
        .template_create(
            e.h,
            MealTemplateInput {
                name: "Week plan".into(),
                days: 3,
                note: None,
                cells: vec![
                    cell(1, dinner, 0, Some(recipe.id), None),
                    cell(1, dinner, 1, None, Some("Salad")),
                    cell(2, lunch, 0, None, Some("Sandwiches")),
                    cell(3, dinner, 0, Some(recipe.id), None),
                ],
            },
        )
        .await
        .unwrap()
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn preview_lists_the_existing_entries_per_slot(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let lunch = e.slot("Lunch").await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    let t = template_with_three_days(&e, &r).await;
    e.free(d(10), dinner, "Out to eat").await;
    e.free(d(10), dinner, "Ice cream").await;
    let p = e
        .svc
        .template_preview(e.h, t.id, TemplatePreviewReq { start: d(10) })
        .await
        .unwrap();
    assert_eq!(
        p.targets.len(),
        3,
        "day 1 dinner, day 2 lunch, day 3 dinner"
    );
    let t1 = &p.targets[0];
    assert_eq!((t1.day, t1.date, t1.slot_id), (1, d(10), dinner));
    assert_eq!(t1.items.len(), 2);
    assert_eq!(
        t1.existing
            .iter()
            .map(|x| x.title.as_str())
            .collect::<Vec<_>>(),
        ["Out to eat", "Ice cream"]
    );
    assert!(t1.slot_visible);
    let t2 = &p.targets[1];
    assert_eq!((t2.date, t2.slot_id, t2.existing.len()), (d(11), lunch, 0));
    assert!(!t2.slot_visible, "Lunch is hidden but still listed");
    assert_eq!(p.targets[2].date, d(12));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn apply_replaces_or_adds_per_slot_and_undoes(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    let t = template_with_three_days(&e, &r).await;
    // Day 1 (the 10th) holds a free-text entry the user wants to keep; day 3 (the 12th) holds a
    // planned recipe that should be replaced.
    let keep = e.free(d(10), dinner, "Grandma's dinner").await;
    let old = e.plan(d(12), dinner, &r, Some(8)).await;
    e.svc
        .generate(
            e.h,
            e.actor,
            GenerateReq {
                entry_ids: Some(vec![old.id]),
                from: None,
                to: None,
            },
        )
        .await
        .unwrap();

    let res = e
        .svc
        .template_apply(
            e.h,
            e.actor,
            t.id,
            TemplateApplyReq {
                start: d(10),
                add_slots: vec![SlotRef {
                    date: d(10),
                    slot_id: dinner,
                }],
            },
        )
        .await
        .unwrap();
    let change = res.change_id.unwrap();
    let range = e.svc.plan_range(e.h, d(10), d(12)).await.unwrap();
    let day10: Vec<_> = range
        .iter()
        .filter(|x| x.date == d(10))
        .map(|x| x.title.as_str())
        .collect();
    assert_eq!(
        day10,
        ["Grandma's dinner", "Chili", "Salad"],
        "added after the kept entry"
    );
    assert!(range.iter().any(|x| x.id == keep.id));
    let day12: Vec<_> = range.iter().filter(|x| x.date == d(12)).collect();
    assert_eq!(day12.len(), 1);
    assert_ne!(day12[0].id, old.id, "replaced");
    assert_eq!(
        day12[0].servings,
        Some(4),
        "template items default to the recipe's servings"
    );
    assert_eq!(res.entries.len(), 4);
    // The replaced entry had unchecked grocery contributions: the impact says so.
    assert!(
        res.grocery_impact
            .iter()
            .any(|i| i.text == "onion" && i.remove),
        "{:?}",
        res.grocery_impact
    );

    e.svc.undo(e.h, e.actor, change).await.unwrap();
    let range = e.svc.plan_range(e.h, d(10), d(12)).await.unwrap();
    assert_eq!(
        range.iter().map(|x| x.title.as_str()).collect::<Vec<_>>(),
        ["Grandma's dinner", "Chili"]
    );
    assert!(
        range
            .iter()
            .any(|x| x.id == old.id && x.servings == Some(8)),
        "the replaced entry is back as it was"
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn apply_refuses_the_past_and_the_far_future(pool: PgPool) {
    let e = env(pool).await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    let t = template_with_three_days(&e, &r).await;
    for start in [d(7), d(22)] {
        let res = e
            .svc
            .template_apply(
                e.h,
                e.actor,
                t.id,
                TemplateApplyReq {
                    start,
                    add_slots: vec![],
                },
            )
            .await;
        assert!(
            matches!(res, Err(MealsError::Invalid { field: "date", .. })),
            "{start}"
        );
    }
    assert!(
        e.svc.plan_range(e.h, d(1), d(30)).await.unwrap().is_empty(),
        "nothing was written"
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn from_plan_keeps_two_entries_of_a_slot_in_order(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    e.plan(d(10), dinner, &r, Some(6)).await;
    e.free(d(10), dinner, "Salad").await;
    e.free(d(12), dinner, "Pizza").await;
    let lunch = e.slot("Lunch").await;
    e.free(d(11), lunch, "Not saved").await;
    let t = e
        .svc
        .template_from_plan(
            e.h,
            TemplateFromPlanReq {
                name: "Saved".into(),
                from: d(10),
                to: d(12),
                slot_ids: vec![dinner],
            },
        )
        .await
        .unwrap();
    assert_eq!(t.days, 3);
    let cells: Vec<_> = t
        .cells
        .iter()
        .map(|c| (c.day, c.title.as_str(), c.servings))
        .collect();
    assert_eq!(
        cells,
        [
            (1, "Chili", Some(6)),
            (1, "Salad", None),
            (3, "Pizza", None)
        ]
    );
    assert!(matches!(
        e.svc
            .template_from_plan(
                e.h,
                TemplateFromPlanReq {
                    name: "Bad".into(),
                    from: d(10),
                    to: chrono::NaiveDate::from_ymd_opt(2028, 1, 1).unwrap(),
                    slot_ids: vec![dinner]
                }
            )
            .await,
        Err(MealsError::Invalid { .. })
    ));
    let list = e.svc.template_list(e.h).await.unwrap();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].slot_ids, vec![dinner]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn duplicate_and_edits_stay_apart(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let r = e.recipe("Chili", Some(4), &["1 onion"]).await;
    let t = template_with_three_days(&e, &r).await;
    let copy = e
        .svc
        .template_duplicate(
            e.h,
            t.id,
            TemplateDuplicateReq {
                name: "Week plan 2".into(),
            },
        )
        .await
        .unwrap();
    assert_ne!(copy.id, t.id);
    assert_eq!(copy.cells.len(), t.cells.len());
    assert_eq!(copy.name, "Week plan 2");

    let applied = e
        .svc
        .template_apply(
            e.h,
            e.actor,
            t.id,
            TemplateApplyReq {
                start: d(10),
                add_slots: vec![],
            },
        )
        .await
        .unwrap();
    // Edit the template (drop everything); the applied entries and the copy are unchanged.
    let edited = e
        .svc
        .template_update(
            e.h,
            t.id,
            MealTemplateInput {
                name: "Emptied".into(),
                days: 1,
                note: Some("n".into()),
                cells: vec![],
            },
            t.version,
        )
        .await
        .unwrap();
    assert!(edited.cells.is_empty() && edited.version > t.version);
    let range = e.svc.plan_range(e.h, d(10), d(12)).await.unwrap();
    assert_eq!(range.len(), applied.entries.len());
    assert_eq!(
        e.svc.template_get(e.h, copy.id).await.unwrap().cells.len(),
        4
    );
    // And an entry's later edit does not reach the template.
    let first = range[0].clone();
    e.svc
        .plan_update(
            e.h,
            e.actor,
            first.id,
            MealPlanEntryPatch {
                servings: Some(9),
                ..pp()
            },
            first.version,
        )
        .await
        .unwrap();
    assert!(
        e.svc
            .template_get(e.h, copy.id)
            .await
            .unwrap()
            .cells
            .iter()
            .all(|c| c.servings.is_none())
    );
    // Stale edits conflict; cells outside the days and unknown slots are invalid.
    let stale = e
        .svc
        .template_update(
            e.h,
            t.id,
            MealTemplateInput {
                name: "x".into(),
                days: 1,
                note: None,
                cells: vec![],
            },
            t.version,
        )
        .await;
    assert!(matches!(stale, Err(MealsError::Conflict(_))));
    let bad = e
        .svc
        .template_create(
            e.h,
            MealTemplateInput {
                name: "Bad".into(),
                days: 2,
                note: None,
                cells: vec![TemplateCellInput {
                    day: 3,
                    slot_id: dinner,
                    ord: 0,
                    recipe_id: None,
                    free_text: Some("x".into()),
                    servings: None,
                }],
            },
        )
        .await;
    assert!(matches!(bad, Err(MealsError::Invalid { .. })));
    e.svc.template_delete(e.h, copy.id).await.unwrap();
    assert!(matches!(
        e.svc.template_get(e.h, copy.id).await,
        Err(MealsError::NotFound)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn slots_in_use_cannot_be_removed(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    e.free(d(10), dinner, "Pizza").await;
    let mut slots = e.svc.list_slots(e.h).await.unwrap();
    assert_eq!(
        slots
            .iter()
            .map(|s| (s.name.as_str(), s.time.as_str(), s.visible))
            .collect::<Vec<_>>(),
        [
            ("Breakfast", "07:30", false),
            ("Lunch", "12:00", false),
            ("Snack", "15:30", false),
            ("Dinner", "18:00", true)
        ]
    );
    let lunch = slots.iter().position(|s| s.name == "Lunch").unwrap();
    slots.remove(lunch);
    assert!(
        e.svc.replace_slots(e.h, slots.clone()).await.is_ok(),
        "an unused slot can go"
    );
    slots.retain(|s| s.name != "Dinner");
    assert!(matches!(
        e.svc.replace_slots(e.h, slots).await,
        Err(MealsError::SlotInUse)
    ));
    let bad = e
        .svc
        .replace_slots(
            e.h,
            vec![MealSlotView {
                id: None,
                name: "Brunch".into(),
                time: "9:5".into(),
                visible: true,
                ord: 0,
            }],
        )
        .await;
    assert!(matches!(
        bad,
        Err(MealsError::Invalid { field: "time", .. })
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn dashboard_meals_show_todays_visible_slots(pool: PgPool) {
    let e = env(pool).await;
    let dinner = e.dinner().await;
    let breakfast = e.slot("Breakfast").await;
    e.free(d(9), dinner, "Tacos").await;
    e.free(d(9), dinner, "Churros").await;
    e.free(d(9), breakfast, "Hidden toast").await;
    e.free(d(10), dinner, "Tomorrow").await;
    let m = e.svc.dashboard_meals(e.h).await.unwrap();
    assert_eq!(
        m.iter()
            .map(|x| (x.slot_name.as_str(), x.time.as_str(), x.title.as_str()))
            .collect::<Vec<_>>(),
        [("Dinner", "18:00", "Tacos"), ("Dinner", "18:00", "Churros")]
    );
    let _ = ActorRef::default();
}
