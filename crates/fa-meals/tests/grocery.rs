//! Generation, reconcile, undo, grouping and the living list over a real Postgres.

mod common;

use chrono::Duration;
use common::*;
use fa_domain::*;
use fa_meals::{ActorRef, MealsError};
use sqlx::PgPool;

async fn pasta_and_stew(e: &Env) -> (RecipeView, RecipeView) {
    let a = e
        .recipe(
            "Tomato Pasta",
            Some(4),
            &["2 tomatoes", "1 onion", "200 g spaghetti", "1 tsp salt"],
        )
        .await;
    let b = e
        .recipe(
            "Milk Stew",
            Some(2),
            &["3 tomatoes", "500 ml milk", "1/2 tsp salt"],
        )
        .await;
    (a, b)
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn generate_merges_scales_and_sorts_staples(pool: PgPool) {
    let e = env(pool).await;
    e.staple("salt").await;
    let (a, b) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, Some(6)).await; // 4 -> 6 servings: x1.5
    e.plan(d(11), dinner, &b, None).await;
    let r = e.generate_range(9, 12).await;
    // tomato, onion, spaghetti and milk are new; the second tomato line merges.
    assert_eq!((r.added, r.merged), (4, 1), "{r:?}");
    assert_eq!(r.staples, 2, "salt is created once and merged once");

    let view = e.svc.grocery_view(e.h).await.unwrap();
    let tomato = view.items.iter().find(|i| i.text == "tomato").unwrap();
    assert_eq!(
        (tomato.qty, tomato.unit.as_deref()),
        (Some(6.0), Some("count"))
    );
    assert_eq!(tomato.display, "6");
    assert_eq!(tomato.sources.len(), 2);
    let onion = view.items.iter().find(|i| i.text == "onion").unwrap();
    assert_eq!(onion.qty, Some(1.5));
    let pasta = view.items.iter().find(|i| i.text == "spaghetti").unwrap();
    assert_eq!((pasta.qty, pasta.display.as_str()), (Some(300.0), "300 g"));
    let milk = view.items.iter().find(|i| i.text == "milk").unwrap();
    assert_eq!((milk.qty, milk.unit.as_deref()), (Some(500.0), Some("ml")));
    assert_eq!(milk.display, "500 ml");
    // Salt is a staple: not on the list, in the staples section with both meals.
    assert!(view.items.iter().all(|i| i.text != "salt"));
    assert_eq!(view.staples.len(), 1);
    assert_eq!(view.staples[0].name, "salt");
    assert_eq!(view.staples[0].needed_for.len(), 2);
    assert!(
        view.items
            .iter()
            .all(|i| !i.manual && i.section == GrocerySection::List)
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn generate_twice_adds_nothing(pool: PgPool) {
    let e = env(pool).await;
    e.staple("salt").await;
    let (a, b) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    e.plan(d(11), dinner, &b, None).await;
    e.generate_range(9, 12).await;
    let before = e.svc.grocery_view(e.h).await.unwrap();
    let again = e.generate_range(9, 12).await;
    assert_eq!((again.added, again.merged, again.staples), (0, 0, 0));
    let after = e.svc.grocery_view(e.h).await.unwrap();
    let key = |v: &GroceryView| {
        v.items
            .iter()
            .map(|i| (i.text.clone(), i.qty, i.unit.clone(), i.sources.len()))
            .collect::<Vec<_>>()
    };
    assert_eq!(key(&before), key(&after));
    assert_eq!(
        before.staples[0].needed_for.len(),
        after.staples[0].needed_for.len()
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn generate_skips_hidden_slots_and_free_text(pool: PgPool) {
    let e = env(pool).await;
    let (a, _) = pasta_and_stew(&e).await;
    let breakfast = e.slot("Breakfast").await;
    let dinner = e.dinner().await;
    e.plan(d(10), breakfast, &a, None).await;
    e.free(d(10), dinner, "Leftovers").await;
    let r = e.generate_range(9, 12).await;
    assert_eq!((r.added, r.merged, r.staples), (0, 0, 0));
    assert!(e.items().await.is_empty());
    // Picking the hidden entry by id is allowed.
    let picked = e.svc.plan_range(e.h, d(10), d(10)).await.unwrap();
    let hidden = picked.iter().find(|p| p.slot_id == breakfast).unwrap();
    let r = e
        .svc
        .generate(
            e.h,
            e.actor,
            GenerateReq {
                entry_ids: Some(vec![hidden.id]),
                from: None,
                to: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(r.added, 4);
    // Both forms at once is invalid.
    let bad = e
        .svc
        .generate(
            e.h,
            e.actor,
            GenerateReq {
                entry_ids: Some(vec![hidden.id]),
                from: Some(d(9)),
                to: Some(d(10)),
            },
        )
        .await;
    assert!(matches!(bad, Err(MealsError::Invalid { .. })));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn generate_undo_restores_and_checks_who_and_when(pool: PgPool) {
    let e = env(pool).await;
    let (a, b) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    let first = e.generate_range(9, 12).await;
    let tomato_before = e.item("tomato").await.unwrap();
    assert_eq!(tomato_before.qty, Some(2.0));
    e.plan(d(11), dinner, &b, None).await;
    let second = e.generate_range(9, 12).await;
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(5.0));

    // A child who did not make the change may not undo it; another adult may.
    let kid = ActorRef {
        member: Some(e.kid),
        device: None,
    };
    assert!(matches!(
        e.svc.undo(e.h, kid, second.change_id).await,
        Err(MealsError::NotFound)
    ));
    e.svc.undo(e.h, e.actor, second.change_id).await.unwrap();
    let tomato = e.item("tomato").await.unwrap();
    assert_eq!(tomato.qty, Some(2.0));
    assert_eq!(tomato.sources.len(), 1);
    assert!(e.item("milk").await.is_none());
    assert!(matches!(
        e.svc.undo(e.h, e.actor, second.change_id).await,
        Err(MealsError::AlreadyUndone)
    ));
    e.svc.undo(e.h, e.actor, first.change_id).await.unwrap();
    assert!(e.items().await.is_empty());

    // Past ten minutes the undo is gone.
    let third = e.generate_range(9, 12).await;
    e.at(base_time() + Duration::minutes(11));
    assert!(matches!(
        e.svc.undo(e.h, e.actor, third.change_id).await,
        Err(MealsError::Expired)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn same_device_may_undo(pool: PgPool) {
    let e = env(pool).await;
    let (a, _) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let dev = DeviceId::new();
    sqlx::query(
        "INSERT INTO device (id, household_id, name, kind, token_hash)
         VALUES ($1, $2, 'Kitchen', 'dashboard', 'x')",
    )
    .bind(dev.as_uuid())
    .bind(e.h.as_uuid())
    .execute(&mut *tx)
    .await
    .unwrap();
    tx.commit().await.unwrap();
    let device = ActorRef {
        member: None,
        device: Some(dev),
    };
    let r = e
        .svc
        .generate(
            e.h,
            device,
            GenerateReq {
                entry_ids: None,
                from: Some(d(9)),
                to: Some(d(12)),
            },
        )
        .await
        .unwrap();
    e.svc.undo(e.h, device, r.change_id).await.unwrap();
    assert!(e.items().await.is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn reconcile_after_delete_reduces_and_removes(pool: PgPool) {
    let e = env(pool).await;
    let (a, b) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    let eb = e.plan(d(11), dinner, &b, None).await;
    e.generate_range(9, 12).await;
    let paper = e.add_item("paper towels").await;
    // The onion is checked off: it stays whatever the plan does.
    let onion = e.item("onion").await.unwrap();
    e.svc.grocery_check(e.h, e.actor, onion.id).await.unwrap();
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(5.0));

    let res = e.svc.plan_delete(e.h, e.actor, eb.id).await.unwrap();
    assert!(res.entries.is_empty());
    let tomato_imp = res
        .grocery_impact
        .iter()
        .find(|i| i.text == "tomato")
        .unwrap();
    assert_eq!(
        (tomato_imp.from_qty, tomato_imp.to_qty, tomato_imp.remove),
        (Some(5.0), Some(2.0), false)
    );
    let milk_imp = res
        .grocery_impact
        .iter()
        .find(|i| i.text == "milk")
        .unwrap();
    assert!(milk_imp.remove && milk_imp.to_qty.is_none());
    assert!(res.grocery_impact.iter().all(|i| i.entry_id == eb.id));
    // The list is not touched until the user says yes.
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(5.0));

    let rec = e
        .svc
        .reconcile(
            e.h,
            e.actor,
            ReconcileReq {
                entry_ids: vec![eb.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(rec.changed, 3, "tomato and salt reduced, milk removed");
    assert!((e.item("salt").await.unwrap().qty.unwrap() - 4.92892).abs() < 1e-6);
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(2.0));
    assert!(e.item("milk").await.is_none());
    assert!(e.item("onion").await.unwrap().checked_at.is_some());
    assert!(e.items().await.iter().any(|i| i.id == paper.id));
    // Reconciling again changes nothing: the parked contribution is consumed.
    let again = e
        .svc
        .reconcile(
            e.h,
            e.actor,
            ReconcileReq {
                entry_ids: vec![eb.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(again.changed, 0);

    // Undo puts the milk and the tomatoes back.
    e.svc.undo(e.h, e.actor, rec.change_id).await.unwrap();
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(5.0));
    assert_eq!(e.item("milk").await.unwrap().qty, Some(500.0));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn checked_items_are_not_in_the_impact(pool: PgPool) {
    let e = env(pool).await;
    let (a, _) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    let ea = e.plan(d(10), dinner, &a, None).await;
    e.generate_range(9, 12).await;
    for t in ["tomato", "onion", "spaghetti", "salt"] {
        let i = e.item(t).await.unwrap();
        e.svc.grocery_check(e.h, e.actor, i.id).await.unwrap();
    }
    let res = e.svc.plan_delete(e.h, e.actor, ea.id).await.unwrap();
    assert!(res.grocery_impact.is_empty(), "{:?}", res.grocery_impact);
    let rec = e
        .svc
        .reconcile(
            e.h,
            e.actor,
            ReconcileReq {
                entry_ids: vec![ea.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(rec.changed, 0);
    assert_eq!(e.item("tomato").await.unwrap().qty, Some(2.0));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn servings_and_moves_change_the_impact(pool: PgPool) {
    let e = env(pool).await;
    let (a, _) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    let ea = e.plan(d(10), dinner, &a, None).await; // 4 servings
    e.generate_range(9, 12).await;
    assert_eq!(e.item("spaghetti").await.unwrap().qty, Some(200.0));

    // Doubling the servings doubles the contribution.
    let res = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            ea.id,
            MealPlanEntryPatch {
                servings: Some(8),
                ..pp()
            },
            ea.version,
        )
        .await
        .unwrap();
    let sp = res
        .grocery_impact
        .iter()
        .find(|i| i.text == "spaghetti")
        .unwrap();
    assert_eq!((sp.from_qty, sp.to_qty), (Some(200.0), Some(400.0)));
    assert!(!sp.remove);
    e.svc
        .reconcile(
            e.h,
            e.actor,
            ReconcileReq {
                entry_ids: vec![ea.id],
            },
        )
        .await
        .unwrap();
    assert_eq!(e.item("spaghetti").await.unwrap().qty, Some(400.0));

    // A move inside the generated range asks nothing; one outside removes the contribution.
    let entry = res.entries[0].clone();
    let inside = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            entry.id,
            MealPlanEntryPatch {
                date: Some(d(11)),
                ..pp()
            },
            entry.version,
        )
        .await
        .unwrap();
    assert!(inside.grocery_impact.is_empty());
    assert!(inside.change_id.is_some(), "a move is one undoable change");
    let entry = inside.entries[0].clone();
    let outside = e
        .svc
        .plan_update(
            e.h,
            e.actor,
            entry.id,
            MealPlanEntryPatch {
                date: Some(d(16)),
                ..pp()
            },
            entry.version,
        )
        .await
        .unwrap();
    assert!(
        outside
            .grocery_impact
            .iter()
            .any(|i| i.text == "spaghetti" && i.remove)
    );
    e.svc
        .reconcile(
            e.h,
            e.actor,
            ReconcileReq {
                entry_ids: vec![entry.id],
            },
        )
        .await
        .unwrap();
    assert!(e.items().await.is_empty());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn volume_and_weight_stay_two_quantities(pool: PgPool) {
    let e = env(pool).await;
    let a = e.recipe("Butter A", Some(2), &["2 tbsp butter"]).await;
    let b = e
        .recipe("Butter B", Some(2), &["100 g butter", "1 tbsp butter"])
        .await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    e.plan(d(11), dinner, &b, None).await;
    e.generate_range(9, 12).await;
    let items = e.items().await;
    assert_eq!(items.len(), 1);
    let butter = &items[0];
    assert_eq!(
        (butter.unit.as_deref(), butter.unit2.as_deref()),
        (Some("ml"), Some("g"))
    );
    assert!((butter.qty.unwrap() - 3.0 * 14.7868).abs() < 1e-6);
    assert_eq!(butter.qty2, Some(100.0));
    assert_eq!(butter.display, "45 ml + 100 g");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn grouping_follows_each_stores_aisle_order(pool: PgPool) {
    let e = env(pool).await;
    let aisles = e.svc.list_aisles(e.h).await.unwrap();
    let id = |n: &str| aisles.iter().find(|a| a.name == n).unwrap().id.unwrap();
    let (produce, dairy, bakery) = (id("Produce"), id("Dairy & Eggs"), id("Bakery"));
    let stores = e
        .svc
        .replace_stores(
            e.h,
            vec![StoreView {
                id: None,
                name: "Corner Market".into(),
                ord: 0,
            }],
        )
        .await
        .unwrap();
    let store = stores[0].id.unwrap();
    // This store walks Dairy, then Bakery, then Produce, and hides nothing yet.
    let mut order: Vec<StoreAisleView> = e.svc.store_aisles(e.h, store).await.unwrap();
    let rest: Vec<AisleId> = order
        .iter()
        .map(|a| a.aisle_id)
        .filter(|a| ![dairy, bakery, produce].contains(a))
        .collect();
    order = [dairy, bakery, produce]
        .into_iter()
        .chain(rest)
        .enumerate()
        .map(|(i, a)| StoreAisleView {
            aisle_id: a,
            ord: i as u32,
            hidden: false,
        })
        .collect();
    e.svc
        .set_store_aisles(e.h, store, order.clone())
        .await
        .unwrap();
    // Ingredients that default to this store.
    for name in ["milk", "tomato", "bread"] {
        let ing = e
            .svc
            .list_ingredients(e.h, None)
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.canonical_name == name)
            .unwrap();
        e.svc
            .update_ingredient(
                e.h,
                ing.id,
                IngredientInput {
                    canonical_name: ing.canonical_name,
                    aliases: ing.aliases,
                    is_staple: ing.is_staple,
                    aisle_id: ing.aisle_id,
                    default_store_id: Some(store),
                    default_unit: ing.default_unit,
                },
            )
            .await
            .unwrap();
    }
    e.add_item("tomato").await;
    e.add_item("bread").await;
    e.add_item("milk").await;
    e.add_item("banana").await; // "Any store": household order, Produce
    e.add_item("paper towels").await; // no aisle, any store
    let texts = |v: Vec<GroceryItemView>| v.into_iter().map(|i| i.text).collect::<Vec<_>>();
    let view = e.svc.grocery_view(e.h).await.unwrap();
    // The store's own order first, then "Any store".
    assert_eq!(
        texts(view.items.clone()),
        ["milk", "bread", "tomato", "banana", "paper towels"]
    );

    // The store hides Dairy: the milk goes to that store's "Other", after the produce.
    let hide: Vec<StoreAisleView> = order
        .iter()
        .map(|a| StoreAisleView {
            hidden: a.aisle_id == dairy,
            ..a.clone()
        })
        .collect();
    e.svc.set_store_aisles(e.h, store, hide).await.unwrap();
    let view = e.svc.grocery_view(e.h).await.unwrap();
    assert_eq!(
        texts(view.items.clone()),
        ["bread", "tomato", "milk", "banana", "paper towels"]
    );
    let milk = view.items.iter().find(|i| i.text == "milk").unwrap();
    assert_eq!(milk.aisle_id, None, "hidden aisle shows as Other");
    assert_eq!(milk.store_id, Some(store));
    let banana = view.items.iter().find(|i| i.text == "banana").unwrap();
    assert_eq!(banana.aisle_id, Some(produce));
    assert_eq!(view.stores.len(), 1);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn any_store_uses_the_household_order(pool: PgPool) {
    let e = env(pool).await;
    e.add_item("bread").await;
    e.add_item("banana").await;
    e.add_item("milk").await;
    let v = e.svc.grocery_view(e.h).await.unwrap();
    // Household aisle order: Produce, Bakery, ..., Dairy & Eggs.
    let order: Vec<&str> = v.items.iter().map(|i| i.text.as_str()).collect();
    assert_eq!(order, ["banana", "bread", "milk"]);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn adds_are_idempotent_and_normalised(pool: PgPool) {
    let e = env(pool).await;
    let id = GroceryItemId::new();
    let req = GroceryAddReq {
        id: Some(id),
        text: "2 cups tomatoes".into(),
        qty: None,
        unit: None,
        store_id: None,
    };
    let (first, created) = e.svc.grocery_add(e.h, e.actor, req.clone()).await.unwrap();
    assert!(created);
    assert_eq!(first.id, id);
    assert_eq!(first.text, "tomato", "matched to the catalogue");
    assert_eq!(
        (first.qty, first.unit.as_deref()),
        (Some(480.0), Some("ml"))
    );
    assert!(first.manual);
    assert!(first.aisle_id.is_some());
    let (again, created) = e.svc.grocery_add(e.h, e.actor, req).await.unwrap();
    assert!(!created);
    assert_eq!(again.id, id);
    assert_eq!(e.items().await.len(), 1);
    // Free text that matches nothing stays as typed.
    let odd = e.add_item("Birthday candles").await;
    assert_eq!(odd.text, "Birthday candles");
    assert_eq!(odd.aisle_id, None);
    assert!(matches!(
        e.svc
            .grocery_add(
                e.h,
                e.actor,
                GroceryAddReq {
                    id: None,
                    text: "  ".into(),
                    qty: None,
                    unit: None,
                    store_id: None
                }
            )
            .await,
        Err(MealsError::Invalid { field: "text", .. })
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn check_off_wins_over_a_stale_version(pool: PgPool) {
    let e = env(pool).await;
    let item = e.add_item("milk").await;
    // An adult edits while the kiosk still holds the old version.
    let edited = e
        .svc
        .grocery_update(
            e.h,
            item.id,
            GroceryItemPatch {
                qty: Some(2.0),
                unit: Some("l".into()),
                ..gp()
            },
            item.version,
        )
        .await
        .unwrap();
    assert!(edited.version > item.version);
    assert_eq!(
        (edited.qty, edited.unit.as_deref()),
        (Some(2000.0), Some("ml"))
    );
    assert_eq!(edited.display, "2 l");
    // The edit with the old version conflicts...
    assert!(matches!(
        e.svc
            .grocery_update(
                e.h,
                item.id,
                GroceryItemPatch {
                    text: Some("oat milk".into()),
                    ..gp()
                },
                item.version
            )
            .await,
        Err(MealsError::Conflict(_))
    ));
    // ...but the check-off has no version to be stale against.
    let kid = ActorRef {
        member: Some(e.kid),
        device: None,
    };
    let checked = e.svc.grocery_check(e.h, kid, item.id).await.unwrap();
    assert!(checked.checked_at.is_some());
    let twice = e.svc.grocery_check(e.h, kid, item.id).await.unwrap();
    assert_eq!(twice.checked_at, checked.checked_at);
    assert_eq!(twice.version, checked.version);
    let un = e.svc.grocery_uncheck(e.h, item.id).await.unwrap();
    assert!(un.checked_at.is_none());
    // A change to a deleted item is reported as not found.
    e.svc.grocery_delete(e.h, item.id).await.unwrap();
    assert!(matches!(
        e.svc.grocery_check(e.h, kid, item.id).await,
        Err(MealsError::NotFound)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn low_moves_a_staple_onto_the_list(pool: PgPool) {
    let e = env(pool).await;
    e.staple("salt").await;
    let (a, _) = pasta_and_stew(&e).await;
    let dinner = e.dinner().await;
    e.plan(d(10), dinner, &a, None).await;
    e.generate_range(9, 12).await;
    let staple = e.svc.grocery_view(e.h).await.unwrap().staples.remove(0);
    let low = e.svc.grocery_low(e.h, staple.item_id).await.unwrap();
    assert_eq!(low.section, GrocerySection::List);
    assert!(e.item("salt").await.is_some());
    assert!(e.svc.grocery_view(e.h).await.unwrap().staples.is_empty());
    // On the list already: unchanged.
    let again = e.svc.grocery_low(e.h, staple.item_id).await.unwrap();
    assert_eq!(again.version, low.version);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn checked_items_archive_after_the_configured_hours(pool: PgPool) {
    let e = env(pool).await;
    // `checked_at` is the database's clock, so this test runs on real time.
    let now = chrono::Utc::now();
    e.at(now);
    let milk = e.add_item("milk").await;
    let eggs = e.add_item("egg").await;
    e.svc.grocery_check(e.h, e.actor, milk.id).await.unwrap();
    // 23 hours later: still there. 25 hours later: archived (default 24).
    e.at(now + Duration::hours(23));
    assert_eq!(e.svc.archive_household(e.h).await.unwrap(), 0);
    assert_eq!(e.items().await.len(), 2);
    e.at(now + Duration::hours(25));
    assert_eq!(e.svc.archive_household(e.h).await.unwrap(), 1);
    let left = e.items().await;
    assert_eq!(left.len(), 1);
    assert_eq!(left[0].id, eggs.id);
    // The hours are a setting.
    e.svc
        .update_settings(
            e.h,
            MealsSettingsPatch {
                grocery_archive_hours: Some(1),
                ..sp()
            },
        )
        .await
        .unwrap();
    e.svc.grocery_check(e.h, e.actor, eggs.id).await.unwrap();
    e.at(now + Duration::hours(27));
    assert_eq!(e.svc.archive_all().await.unwrap(), 1);
    assert!(e.items().await.is_empty());
}
