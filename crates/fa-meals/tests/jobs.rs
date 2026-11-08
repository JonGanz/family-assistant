//! The jobs, `ensure_defaults` and the stub endpoints, over a real Postgres.

mod common;

use std::sync::Arc;

use common::*;
use fa_domain::*;
use fa_jobs::{JobContext, JobHandler};
use fa_meals::{ImageOutcome, LlmOutcome, MealsService, default_schedules, handlers, seed_data};
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;

fn handler(svc: &MealsService, kind: &str) -> Arc<dyn JobHandler> {
    handlers(Arc::new(svc.clone()))
        .into_iter()
        .find(|h| h.kind() == kind)
        .unwrap()
}

fn ctx(e: &Env) -> JobContext {
    JobContext {
        store: e.store.clone(),
        backup_dir: None,
    }
}

async fn jobs(e: &Env, kind: &str) -> Vec<Value> {
    let mut tx = e.store.tx_system().await.unwrap();
    let rows: Vec<Value> =
        sqlx::query_scalar("SELECT payload FROM job WHERE kind = $1 ORDER BY created_at")
            .bind(kind)
            .fetch_all(&mut *tx)
            .await
            .unwrap();
    rows
}

async fn counts(e: &Env) -> (i64, i64, i64, i64, i64) {
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    (
        IngredientRepo::count(&mut tx).await.unwrap(),
        AisleRepo::count(&mut tx).await.unwrap(),
        MealSlotRepo::count(&mut tx).await.unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM grocery_list")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
        sqlx::query_scalar("SELECT count(*) FROM meals_settings")
            .fetch_one(&mut *tx)
            .await
            .unwrap(),
    )
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn ensure_defaults_is_idempotent(pool: PgPool) {
    let e = env_with(
        pool,
        Opts {
            seed: false,
            ..Default::default()
        },
    )
    .await;
    assert_eq!(counts(&e).await, (0, 0, 0, 0, 0));
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let first = fa_meals::ensure_defaults(&mut tx).await.unwrap();
    tx.commit().await.unwrap();
    let (ings, aisles) = (seed_data::ingredients().len(), seed_data::aisles().len());
    assert_eq!(
        first,
        1 + aisles + 4 + ings + 1,
        "settings, aisles, slots, ingredients, list"
    );
    let c = counts(&e).await;
    assert_eq!(c, (ings as i64, aisles as i64, 4, 1, 1));
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    assert_eq!(fa_meals::ensure_defaults(&mut tx).await.unwrap(), 0);
    tx.commit().await.unwrap();
    assert_eq!(counts(&e).await, c);

    // Seeded ingredients carry their aisle, origin and unit.
    let all = e.svc.list_ingredients(e.h, None).await.unwrap();
    assert!(
        all.iter()
            .all(|i| i.origin == IngredientOrigin::Seed && i.aisle_id.is_some())
    );
    assert!(
        all.iter().all(|i| !i.is_staple),
        "staples are the household's choice"
    );
    assert!(all.iter().any(|i| i.default_unit == Some(BaseUnit::Ml)));
    // Only Dinner shows.
    let slots = e.svc.list_slots(e.h).await.unwrap();
    assert_eq!(
        slots
            .iter()
            .filter(|s| s.visible)
            .map(|s| s.name.as_str())
            .collect::<Vec<_>>(),
        ["Dinner"]
    );

    // A household that removed a slot does not get it back.
    let kept: Vec<MealSlotView> = slots.into_iter().filter(|s| s.name != "Snack").collect();
    e.svc.replace_slots(e.h, kept).await.unwrap();
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    assert_eq!(fa_meals::ensure_defaults(&mut tx).await.unwrap(), 0);
    tx.commit().await.unwrap();
    assert_eq!(e.svc.list_slots(e.h).await.unwrap().len(), 3);

    // Another household is seeded on its own; neither sees the other's rows.
    let other = env(e.pool.clone()).await;
    assert_eq!(counts(&other).await, (ings as i64, aisles as i64, 4, 1, 1));
    assert_eq!(counts(&e).await.0, ings as i64);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn lines_queue_the_llm_job_only_when_it_can_run(pool: PgPool) {
    let (url, _) = stub().await;
    let with = env_with(
        pool.clone(),
        Opts {
            llm_url: Some(url),
            ..Default::default()
        },
    )
    .await;
    let r = with
        .recipe(
            "Odd",
            Some(2),
            &["2 tomatoes", "2 dragonfruit", "1 zxqv gizmo"],
        )
        .await;
    let queued = jobs(&with, "ingredient.llm").await;
    assert_eq!(queued.len(), 2, "only the unmatched lines");
    let line = r
        .lines
        .iter()
        .find(|l| l.raw_text == "2 dragonfruit")
        .unwrap();
    assert!(
        queued
            .iter()
            .any(|p| p["line_id"] == json!(line.id) && p["household"] == json!(with.h))
    );
    // A re-save of the same lines queues nothing new (their text is unchanged: kept lines).
    with.svc
        .update_recipe(
            with.h,
            with.actor,
            r.id,
            RecipeInput {
                title: "Odd".into(),
                source_url: None,
                servings: Some(2),
                yield_text: None,
                prep_minutes: None,
                cook_minutes: None,
                total_minutes: None,
                import_method: ImportMethod::Manual,
                tags: vec![],
                note: None,
                image_url: None,
                lines: vec![
                    "2 tomatoes".into(),
                    "2 dragonfruit".into(),
                    "1 zxqv gizmo".into(),
                ],
                steps: vec![],
            },
            r.version,
        )
        .await
        .unwrap();
    assert_eq!(jobs(&with, "ingredient.llm").await.len(), 2);

    // No endpoint: no job. The toggle off: no job.
    let without = env(pool.clone()).await;
    without.recipe("Odd", Some(2), &["2 dragonfruit"]).await;
    assert!(
        jobs(&without, "ingredient.llm")
            .await
            .iter()
            .all(|p| p["household"] != json!(without.h))
    );
    let (url2, _) = stub().await;
    let off = env_with(
        pool,
        Opts {
            llm_url: Some(url2),
            ..Default::default()
        },
    )
    .await;
    off.svc
        .update_settings(
            off.h,
            MealsSettingsPatch {
                ingredient_llm: Some(false),
                ..sp()
            },
        )
        .await
        .unwrap();
    off.recipe("Odd", Some(2), &["2 dragonfruit"]).await;
    assert!(
        jobs(&off, "ingredient.llm")
            .await
            .iter()
            .all(|p| p["household"] != json!(off.h))
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn ingredient_llm_stores_a_proposal_and_never_an_ingredient(pool: PgPool) {
    let (url, state) = stub().await;
    let e = env_with(
        pool,
        Opts {
            llm_url: Some(url),
            ..Default::default()
        },
    )
    .await;
    let r = e
        .recipe(
            "Odd",
            Some(2),
            &["2 dragonfruit", "3 tomatiloz", "1 zxqv gizmo"],
        )
        .await;
    let line = |raw: &str| r.lines.iter().find(|l| l.raw_text == raw).unwrap().id;
    let h = handler(&e.svc, "ingredient.llm");
    let before = counts(&e).await.0;

    // A new ingredient is proposed.
    state.lock().unwrap().reply = json!({"ingredient_id": null, "new_name": "Dragon Fruit", "aisle": "Produce", "confidence": 0.8});
    h.run(
        &ctx(&e),
        json!({"household": e.h, "line_id": line("2 dragonfruit")}),
    )
    .await
    .unwrap();
    let got = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    let l = got
        .lines
        .iter()
        .find(|l| l.raw_text == "2 dragonfruit")
        .unwrap();
    assert_eq!(l.match_state, MatchState::Proposed);
    assert_eq!(l.ingredient_id, None, "a proposal is not a match");
    let p = l.proposal.clone().unwrap();
    assert_eq!(
        (p.new_name.as_deref(), p.aisle.as_deref(), p.ingredient_id),
        (Some("dragon fruit"), Some("Produce"), None)
    );
    assert!((p.confidence - 0.8).abs() < 1e-6);
    assert_eq!(counts(&e).await.0, before, "no ingredient row was written");
    // The request carried the line and candidates, and asked for the schema.
    let req = state.lock().unwrap().requests.last().cloned().unwrap();
    assert!(req.to_string().contains("dragonfruit"));
    assert!(req.to_string().contains("candidates"));

    // An existing candidate is picked.
    let tomato = e
        .svc
        .list_ingredients(e.h, Some("tomato"))
        .await
        .unwrap()
        .into_iter()
        .find(|i| i.canonical_name == "tomato")
        .unwrap();
    state.lock().unwrap().reply =
        json!({"ingredient_id": tomato.id.to_string(), "confidence": 0.9});
    h.run(
        &ctx(&e),
        json!({"household": e.h, "line_id": line("3 tomatiloz")}),
    )
    .await
    .unwrap();
    let got = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    let l = got
        .lines
        .iter()
        .find(|l| l.raw_text == "3 tomatiloz")
        .unwrap();
    assert_eq!(
        (l.match_state, l.ingredient_id),
        (MatchState::Proposed, None)
    );
    assert_eq!(l.proposal.as_ref().unwrap().ingredient_id, Some(tomato.id));

    // An id that was not offered is discarded, and so is an empty answer.
    state.lock().unwrap().reply =
        json!({"ingredient_id": IngredientId::new().to_string(), "confidence": 0.9});
    h.run(
        &ctx(&e),
        json!({"household": e.h, "line_id": line("1 zxqv gizmo")}),
    )
    .await
    .unwrap();
    state.lock().unwrap().reply = json!({"confidence": 0.1});
    h.run(
        &ctx(&e),
        json!({"household": e.h, "line_id": line("1 zxqv gizmo")}),
    )
    .await
    .unwrap();
    let got = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    let l = got
        .lines
        .iter()
        .find(|l| l.raw_text == "1 zxqv gizmo")
        .unwrap();
    assert_eq!(
        (l.match_state, l.proposal.is_none()),
        (MatchState::Unmatched, true)
    );
    assert_eq!(counts(&e).await.0, before);
    // The model's calls were recorded by purpose.
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let n: i64 = sqlx::query_scalar("SELECT count(*) FROM llm_call WHERE purpose = 'ingredient'")
        .fetch_one(&mut *tx)
        .await
        .unwrap();
    assert_eq!(n, 4);
    drop(tx);

    // A bad payload is an error, not a panic.
    assert!(h.run(&ctx(&e), json!({"household": e.h})).await.is_err());
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn ingredient_llm_without_a_client_is_a_no_op(pool: PgPool) {
    let e = env(pool).await;
    let r = e.recipe("Odd", Some(2), &["2 dragonfruit"]).await;
    let id = r.lines[0].id;
    let h = handler(&e.svc, "ingredient.llm");
    h.run(&ctx(&e), json!({"household": e.h, "line_id": id}))
        .await
        .unwrap();
    assert_eq!(
        e.svc.llm_match_line(e.h, id).await.unwrap(),
        LlmOutcome::Skipped
    );
    let got = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    assert_eq!(got.lines[0].match_state, MatchState::Unmatched);

    // An endpoint row with the provider missing, or the toggle off, is a no-op as well.
    let (url, state) = stub().await;
    let on = env_with(
        e.pool.clone(),
        Opts {
            llm_url: Some(url),
            ..Default::default()
        },
    )
    .await;
    let r = on.recipe("Odd", Some(2), &["2 dragonfruit"]).await;
    on.svc
        .update_settings(
            on.h,
            MealsSettingsPatch {
                ingredient_llm: Some(false),
                ..sp()
            },
        )
        .await
        .unwrap();
    state.lock().unwrap().reply = json!({"new_name": "dragon fruit", "confidence": 0.5});
    assert_eq!(
        on.svc.llm_match_line(on.h, r.lines[0].id).await.unwrap(),
        LlmOutcome::Skipped
    );
    assert!(
        state.lock().unwrap().requests.is_empty(),
        "the model was not called"
    );
    // A deleted line is skipped too.
    on.svc.delete_recipe(on.h, r.id).await.unwrap();
    assert_eq!(
        on.svc.llm_match_line(on.h, r.lines[0].id).await.unwrap(),
        LlmOutcome::Skipped
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn recipe_image_job_fetches_resizes_and_stores(pool: PgPool) {
    let (url, state) = stub().await;
    state.lock().unwrap().png = make_png(1200, 900);
    let e = env_with(
        pool.clone(),
        Opts {
            allow_private: true,
            ..Default::default()
        },
    )
    .await;
    let r = e
        .svc
        .create_recipe(
            e.h,
            e.actor,
            RecipeInput {
                title: "Pic".into(),
                source_url: None,
                servings: Some(2),
                yield_text: None,
                prep_minutes: None,
                cook_minutes: None,
                total_minutes: None,
                import_method: ImportMethod::Jsonld,
                tags: vec![],
                note: None,
                image_url: Some(format!("{url}/img.png")),
                lines: vec!["1 egg".into()],
                steps: vec![],
            },
        )
        .await
        .unwrap();
    assert!(!r.has_image);
    let queued = jobs(&e, "recipe.image").await;
    assert_eq!(queued.len(), 1);
    assert_eq!(queued[0]["recipe_id"], json!(r.id));
    let h = handler(&e.svc, "recipe.image");
    h.run(&ctx(&e), queued[0].clone()).await.unwrap();
    let img = e.svc.get_image(e.h, r.id).await.unwrap().unwrap();
    assert!(img.bytes.len() <= 512 * 1024);
    let decoded = image::load_from_memory(&img.bytes).unwrap();
    assert_eq!((decoded.width(), decoded.height()), (800, 600));
    assert!(e.svc.get_recipe(e.h, None, r.id).await.unwrap().has_image);
    // A second run does nothing; an uploaded image is never overwritten.
    assert_eq!(
        e.svc
            .fetch_recipe_image(e.h, r.id, &format!("{url}/img.png"))
            .await
            .unwrap(),
        ImageOutcome::Skipped
    );

    // Not an image, a 404 and a recipe that is gone: no retry.
    let r2 = e.recipe("Pic 2", Some(2), &["1 egg"]).await;
    assert!(matches!(
        e.svc
            .fetch_recipe_image(e.h, r2.id, &format!("{url}/recipe"))
            .await
            .unwrap(),
        ImageOutcome::Refused(_)
    ));
    assert!(matches!(
        e.svc
            .fetch_recipe_image(e.h, r2.id, &format!("{url}/missing"))
            .await
            .unwrap(),
        ImageOutcome::Refused(_)
    ));
    assert!(matches!(
        e.svc
            .fetch_recipe_image(e.h, r2.id, "ftp://example.com/x.png")
            .await
            .unwrap(),
        ImageOutcome::Refused(_)
    ));
    assert_eq!(
        e.svc
            .fetch_recipe_image(e.h, RecipeId::new(), &format!("{url}/img.png"))
            .await
            .unwrap(),
        ImageOutcome::Skipped
    );
    assert!(e.svc.get_image(e.h, r2.id).await.unwrap().is_none());

    // With private hosts off (the default) the stub's address is refused and nothing is stored.
    let closed = env(pool).await;
    let r3 = closed.recipe("Pic 3", Some(2), &["1 egg"]).await;
    let out = closed
        .svc
        .fetch_recipe_image(closed.h, r3.id, &format!("{url}/img.png"))
        .await
        .unwrap();
    assert!(matches!(out, ImageOutcome::Refused(_)), "{out:?}");
    handler(&closed.svc, "recipe.image")
        .run(
            &ctx(&closed),
            json!({"household": closed.h, "recipe_id": r3.id, "url": format!("{url}/img.png")}),
        )
        .await
        .unwrap();
    assert!(
        closed
            .svc
            .get_image(closed.h, r3.id)
            .await
            .unwrap()
            .is_none()
    );
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn archive_job_and_schedules(pool: PgPool) {
    assert_eq!(default_schedules(), [("grocery.archive", "*/15 * * * *")]);
    let e = env(pool).await;
    let kinds: Vec<&str> = handlers(Arc::new(e.svc.clone()))
        .iter()
        .map(|h| h.kind())
        .collect();
    assert_eq!(kinds, ["recipe.image", "ingredient.llm", "grocery.archive"]);
    let now = chrono::Utc::now();
    e.at(now);
    let milk = e.add_item("milk").await;
    e.svc.grocery_check(e.h, e.actor, milk.id).await.unwrap();
    // An undo record long past its expiry is pruned with it.
    let ch = e
        .svc
        .generate(
            e.h,
            e.actor,
            GenerateReq {
                entry_ids: None,
                from: Some(d(9)),
                to: Some(d(10)),
            },
        )
        .await
        .unwrap();
    e.at(now + chrono::Duration::hours(30));
    handler(&e.svc, "grocery.archive")
        .run(&ctx(&e), json!({}))
        .await
        .unwrap();
    assert!(e.items().await.is_empty());
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    assert!(
        MealsChangeRepo::get(&mut tx, ch.change_id)
            .await
            .unwrap()
            .is_none()
    );
}
