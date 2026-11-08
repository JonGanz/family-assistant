//! Recipes, matching, the catalogue, import and images over a real Postgres and a stub site.

mod common;

use common::*;
use fa_domain::recipe::RecipeOrigin;
use fa_domain::*;
use fa_meals::{MealsError, RecipeListQuery};
use serde_json::json;
use sqlx::PgPool;

fn input(title: &str, lines: &[&str]) -> RecipeInput {
    RecipeInput {
        title: title.into(),
        source_url: None,
        servings: Some(4),
        yield_text: None,
        prep_minutes: Some(10),
        cook_minutes: None,
        total_minutes: Some(30),
        import_method: ImportMethod::Manual,
        tags: vec!["quick".into(), "Quick".into(), " ".into()],
        note: None,
        image_url: None,
        lines: lines.iter().map(|s| s.to_string()).collect(),
        steps: vec!["Chop.".into(), "".into(), "Cook.".into()],
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn lines_are_matched_by_rules(pool: PgPool) {
    let e = env(pool).await;
    let r = e
        .svc
        .create_recipe(
            e.h,
            e.actor,
            input(
                "Test Stew",
                &[
                    "2 tomatoes",        // alias / singular
                    "3 tomatoe",         // trigram
                    "1 tsp kosher salt", // descriptor, alias
                    "2 dragonfruit",     // nothing like it
                    "Freshly ground black pepper",
                ],
            ),
        )
        .await
        .unwrap();
    assert_eq!(r.tags, ["quick"], "tags are trimmed and deduplicated");
    assert_eq!(r.steps, ["Chop.", "Cook."]);
    let by = |raw: &str| r.lines.iter().find(|l| l.raw_text == raw).unwrap();
    let t = by("2 tomatoes");
    assert_eq!(
        (t.qty, t.unit.as_deref(), t.item.as_deref()),
        (Some(2.0), None, Some("tomato"))
    );
    assert_eq!(
        (t.match_state, t.ingredient_name.as_deref()),
        (MatchState::Matched, Some("tomato"))
    );
    assert_eq!(by("3 tomatoe").ingredient_name.as_deref(), Some("tomato"));
    let s = by("1 tsp kosher salt");
    assert_eq!(
        (
            s.unit.as_deref(),
            s.item.as_deref(),
            s.ingredient_name.as_deref()
        ),
        (Some("tsp"), Some("salt"), Some("salt"))
    );
    assert_eq!(
        by("Freshly ground black pepper").ingredient_name.as_deref(),
        Some("black pepper")
    );
    let u = by("2 dragonfruit");
    assert_eq!(
        (u.match_state, u.ingredient_id),
        (MatchState::Unmatched, None)
    );
    assert_eq!(u.item.as_deref(), Some("dragonfruit"));
    // raw_text is always the source line.
    assert!(r.lines.iter().all(|l| !l.raw_text.is_empty()));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn matching_a_line_adds_an_alias_and_update_keeps_matches(pool: PgPool) {
    let e = env(pool).await;
    let r = e
        .svc
        .create_recipe(e.h, e.actor, input("Fruit", &["2 dragonfruit", "1 onion"]))
        .await
        .unwrap();
    let line = r
        .lines
        .iter()
        .find(|l| l.raw_text == "2 dragonfruit")
        .unwrap();
    let before = e.svc.list_ingredients(e.h, None).await.unwrap().len();
    let m = e
        .svc
        .match_line(
            e.h,
            r.id,
            line.id,
            LineMatchReq {
                ingredient_id: None,
                new_name: Some("Dragon Fruits".into()),
                aisle_id: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (m.match_state, m.ingredient_name.as_deref()),
        (MatchState::Matched, Some("dragon fruit"))
    );
    let ings = e.svc.list_ingredients(e.h, None).await.unwrap();
    assert_eq!(ings.len(), before + 1);
    let fruit = ings
        .iter()
        .find(|i| i.canonical_name == "dragon fruit")
        .unwrap();
    assert_eq!(fruit.origin, IngredientOrigin::User);
    assert_eq!(
        fruit.aliases,
        ["dragonfruit"],
        "the cleaned item became an alias"
    );
    // The next import matches by rules.
    let again = e
        .svc
        .create_recipe(e.h, e.actor, input("Fruit 2", &["1 dragonfruit"]))
        .await
        .unwrap();
    assert_eq!(again.lines[0].match_state, MatchState::Matched);
    // An update with an unchanged line keeps its match, even where the rules would not.
    let onion = r.lines.iter().find(|l| l.raw_text == "1 onion").unwrap();
    e.svc
        .match_line(
            e.h,
            r.id,
            onion.id,
            LineMatchReq {
                ingredient_id: Some(fruit.id),
                new_name: None,
                aisle_id: None,
            },
        )
        .await
        .unwrap();
    let upd = e
        .svc
        .update_recipe(
            e.h,
            e.actor,
            r.id,
            input("Fruit", &["1 onion", "2 dragonfruit", "1 lemon"]),
            r.version,
        )
        .await
        .unwrap();
    assert_eq!(upd.version, r.version + 1);
    let onion2 = upd.lines.iter().find(|l| l.raw_text == "1 onion").unwrap();
    assert_eq!(onion2.id, onion.id);
    assert_eq!(onion2.ingredient_name.as_deref(), Some("dragon fruit"));
    assert_eq!(upd.lines[0].raw_text, "1 onion");
    assert!(matches!(
        e.svc
            .update_recipe(e.h, e.actor, r.id, input("Fruit", &["1 onion"]), r.version)
            .await,
        Err(MealsError::Conflict(_))
    ));
    // Nothing to match to.
    assert!(matches!(
        e.svc
            .match_line(
                e.h,
                r.id,
                onion.id,
                LineMatchReq {
                    ingredient_id: None,
                    new_name: None,
                    aisle_id: None
                }
            )
            .await,
        Err(MealsError::Invalid { .. })
    ));
    assert!(matches!(
        e.svc
            .match_line(
                e.h,
                again.id,
                onion.id,
                LineMatchReq {
                    ingredient_id: Some(fruit.id),
                    new_name: None,
                    aisle_id: None
                }
            )
            .await,
        Err(MealsError::NotFound)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn library_search_tags_favourites_ratings_and_paging(pool: PgPool) {
    let e = env(pool).await;
    let mut a = input("Apple Pie", &["3 apples", "200 g butter"]);
    a.tags = vec!["dessert".into()];
    let a = e.svc.create_recipe(e.h, e.actor, a).await.unwrap();
    let b = e
        .svc
        .create_recipe(e.h, e.actor, input("Butter Chicken", &["1 chicken breast"]))
        .await
        .unwrap();
    let c = e
        .svc
        .create_recipe(e.h, e.actor, input("Carrot Soup", &["4 carrots"]))
        .await
        .unwrap();
    let q = |s: &str| RecipeListQuery {
        q: Some(s.into()),
        ..Default::default()
    };
    let titles = |p: &RecipePage| p.items.iter().map(|i| i.title.clone()).collect::<Vec<_>>();
    let all = e
        .svc
        .list_recipes(e.h, None, RecipeListQuery::default())
        .await
        .unwrap();
    assert_eq!(titles(&all), ["Apple Pie", "Butter Chicken", "Carrot Soup"]);
    // Title or ingredient.
    assert_eq!(
        titles(&e.svc.list_recipes(e.h, None, q("butter")).await.unwrap()),
        ["Apple Pie", "Butter Chicken"]
    );
    assert_eq!(
        titles(&e.svc.list_recipes(e.h, None, q("carrot")).await.unwrap()),
        ["Carrot Soup"]
    );
    let dessert = e
        .svc
        .list_recipes(
            e.h,
            None,
            RecipeListQuery {
                tag: Some("dessert".into()),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(titles(&dessert), ["Apple Pie"]);
    assert_eq!(e.svc.recipe_tags(e.h).await.unwrap(), ["dessert", "quick"]);

    e.svc.set_favourite(e.h, b.id, true).await.unwrap();
    let fav = e
        .svc
        .list_recipes(
            e.h,
            None,
            RecipeListQuery {
                favourite: Some(true),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(titles(&fav), ["Butter Chicken"]);

    // Ratings are per member; the child may rate; the view shows the viewer's own.
    e.svc.set_rating(e.h, e.kid, c.id, Some(5)).await.unwrap();
    let r = e.svc.set_rating(e.h, e.adult, c.id, Some(3)).await.unwrap();
    assert_eq!((r.avg, r.mine, r.by_member.len()), (Some(4.0), Some(3), 2));
    let page = e
        .svc
        .list_recipes(e.h, Some(e.kid), RecipeListQuery::default())
        .await
        .unwrap();
    let carrot = page
        .items
        .iter()
        .find(|i| i.title == "Carrot Soup")
        .unwrap();
    assert_eq!((carrot.rating_avg, carrot.my_rating), (Some(4.0), Some(5)));
    let r = e.svc.set_rating(e.h, e.kid, c.id, None).await.unwrap();
    assert_eq!((r.avg, r.mine), (Some(3.0), None));
    assert!(matches!(
        e.svc.set_rating(e.h, e.kid, c.id, Some(6)).await,
        Err(MealsError::Invalid { .. })
    ));

    // Paging by cursor.
    let p1 = e
        .svc
        .list_recipes(
            e.h,
            None,
            RecipeListQuery {
                limit: Some(2),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(titles(&p1), ["Apple Pie", "Butter Chicken"]);
    let p2 = e
        .svc
        .list_recipes(
            e.h,
            None,
            RecipeListQuery {
                limit: Some(2),
                before: p1.next_cursor.clone(),
                ..Default::default()
            },
        )
        .await
        .unwrap();
    assert_eq!(titles(&p2), ["Carrot Soup"]);
    assert!(p2.next_cursor.is_none());
    assert!(matches!(
        e.svc
            .list_recipes(
                e.h,
                None,
                RecipeListQuery {
                    before: Some("zz".into()),
                    ..Default::default()
                }
            )
            .await,
        Err(MealsError::Invalid { .. })
    ));
    // Last planned.
    let dinner = e.dinner().await;
    e.plan(d(9), dinner, &a, None).await;
    let page = e
        .svc
        .list_recipes(e.h, None, RecipeListQuery::default())
        .await
        .unwrap();
    assert_eq!(page.items[0].last_planned, Some(d(9)));
    assert_eq!(page.items[1].last_planned, None);
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn recipe_input_is_validated(pool: PgPool) {
    let e = env(pool).await;
    for bad in [
        RecipeInput {
            title: "  ".into(),
            ..input("x", &[])
        },
        RecipeInput {
            servings: Some(0),
            ..input("x", &[])
        },
        RecipeInput {
            servings: Some(101),
            ..input("x", &[])
        },
        RecipeInput {
            source_url: Some("ftp://example.com/a".into()),
            ..input("x", &[])
        },
        RecipeInput {
            image_url: Some("not a url".into()),
            ..input("x", &[])
        },
    ] {
        assert!(matches!(
            e.svc.create_recipe(e.h, e.actor, bad).await,
            Err(MealsError::Invalid { .. })
        ));
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn catalogue_create_update_merge_delete(pool: PgPool) {
    let e = env(pool).await;
    let aisles = e.svc.list_aisles(e.h).await.unwrap();
    let new = e
        .svc
        .create_ingredient(
            e.h,
            IngredientInput {
                canonical_name: "  Urfa Pepper ".into(),
                aliases: vec!["pepper, urfa".into(), "URFA PEPPER".into(), "".into()],
                is_staple: true,
                aisle_id: aisles[0].id,
                default_store_id: None,
                default_unit: Some(BaseUnit::G),
            },
        )
        .await
        .unwrap();
    assert_eq!(new.canonical_name, "urfa pepper");
    assert_eq!(new.aliases, ["pepper, urfa"]);
    assert_eq!(new.origin, IngredientOrigin::User);
    let dup = e
        .svc
        .create_ingredient(
            e.h,
            IngredientInput {
                canonical_name: "urfa pepper".into(),
                aliases: vec![],
                is_staple: false,
                aisle_id: None,
                default_store_id: None,
                default_unit: None,
            },
        )
        .await;
    assert!(matches!(dup, Err(MealsError::Conflict(_))));
    let bad_aisle = e
        .svc
        .create_ingredient(
            e.h,
            IngredientInput {
                canonical_name: "x".into(),
                aliases: vec![],
                is_staple: false,
                aisle_id: Some(AisleId::new()),
                default_store_id: None,
                default_unit: None,
            },
        )
        .await;
    assert!(matches!(
        bad_aisle,
        Err(MealsError::Invalid {
            field: "aisle_id",
            ..
        })
    ));
    let found = e.svc.list_ingredients(e.h, Some("urfa")).await.unwrap();
    assert_eq!(found[0].canonical_name, "urfa pepper");
    // Merge into another.
    let other = e
        .svc
        .list_ingredients(e.h, Some("black pepper"))
        .await
        .unwrap()
        .remove(0);
    let r = e
        .svc
        .create_recipe(e.h, e.actor, input("Rub", &["1 tsp urfa pepper"]))
        .await
        .unwrap();
    assert_eq!(r.lines[0].ingredient_id, Some(new.id));
    let merged = e.svc.merge_ingredient(e.h, new.id, other.id).await.unwrap();
    assert!(merged.aliases.contains(&"urfa pepper".to_string()));
    let r = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    assert_eq!(r.lines[0].ingredient_id, Some(other.id));
    assert!(matches!(
        e.svc.merge_ingredient(e.h, other.id, other.id).await,
        Err(MealsError::Invalid { .. })
    ));
    e.svc.delete_ingredient(e.h, other.id).await.unwrap();
    let r = e.svc.get_recipe(e.h, None, r.id).await.unwrap();
    assert_eq!(r.lines[0].ingredient_id, None, "the link is cleared");
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn settings_aisles_and_stores(pool: PgPool) {
    let e = env(pool).await;
    let s = e.svc.get_settings(e.h).await.unwrap();
    assert_eq!(
        (
            s.grocery_archive_hours,
            s.ingredient_llm,
            s.meal_plan_horizon_weeks
        ),
        (24, true, 2)
    );
    assert!((s.trigram_threshold - 0.45).abs() < 1e-6);
    assert!(
        s.descriptors.contains(&"kosher".to_string()),
        "the seed list in force"
    );
    let s = e
        .svc
        .update_settings(
            e.h,
            MealsSettingsPatch {
                descriptors: Some(vec!["Smoked".into(), "smoked".into()]),
                ingredient_llm: Some(false),
                ..sp()
            },
        )
        .await
        .unwrap();
    assert_eq!(s.descriptors, ["smoked"]);
    assert!(!s.ingredient_llm);
    // A household list replaces the seed list when cleaning.
    let r = e
        .svc
        .create_recipe(e.h, e.actor, input("D", &["1 kosher salt"]))
        .await
        .unwrap();
    assert_eq!(r.lines[0].item.as_deref(), Some("kosher salt"));
    e.svc
        .update_settings(
            e.h,
            MealsSettingsPatch {
                descriptors: Some(vec![]),
                ..sp()
            },
        )
        .await
        .unwrap();
    for bad in [
        MealsSettingsPatch {
            grocery_archive_hours: Some(0),
            ..sp()
        },
        MealsSettingsPatch {
            trigram_threshold: Some(0.1),
            ..sp()
        },
        MealsSettingsPatch {
            meal_plan_horizon_weeks: Some(0),
            ..sp()
        },
    ] {
        assert!(matches!(
            e.svc.update_settings(e.h, bad).await,
            Err(MealsError::Invalid { .. })
        ));
    }
    // Aisles: rename and reorder keep ids; a duplicate name is refused.
    let mut aisles = e.svc.list_aisles(e.h).await.unwrap();
    aisles.truncate(3);
    aisles[0].name = "Fresh".into();
    let id0 = aisles[0].id;
    let out = e.svc.replace_aisles(e.h, aisles.clone()).await.unwrap();
    assert_eq!(out.len(), 3);
    assert_eq!(out[0].id, id0);
    aisles[1].name = "fresh".into();
    assert!(matches!(
        e.svc.replace_aisles(e.h, aisles).await,
        Err(MealsError::Invalid { .. })
    ));
    let stores = e
        .svc
        .replace_stores(
            e.h,
            vec![
                StoreView {
                    id: None,
                    name: "Market".into(),
                    ord: 0,
                },
                StoreView {
                    id: None,
                    name: "Warehouse".into(),
                    ord: 1,
                },
            ],
        )
        .await
        .unwrap();
    assert_eq!(stores.len(), 2);
    let order = e
        .svc
        .store_aisles(e.h, stores[0].id.unwrap())
        .await
        .unwrap();
    assert_eq!(order.len(), 3);
    assert!(order.iter().all(|a| !a.hidden));
    assert!(matches!(
        e.svc.store_aisles(e.h, GroceryStoreId::new()).await,
        Err(MealsError::NotFound)
    ));
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn images_are_resized_and_served(pool: PgPool) {
    let e = env(pool).await;
    let r = e.recipe("Pic", Some(2), &["1 egg"]).await;
    assert!(e.svc.get_image(e.h, r.id).await.unwrap().is_none());
    e.svc
        .put_image(e.h, r.id, make_png(1600, 1000))
        .await
        .unwrap();
    let img = e.svc.get_image(e.h, r.id).await.unwrap().unwrap();
    assert!(img.bytes.len() <= 512 * 1024, "{} bytes", img.bytes.len());
    assert!(img.mime == "image/webp" || img.mime == "image/jpeg");
    let decoded = image::load_from_memory(&img.bytes).unwrap();
    assert!(
        decoded.width().max(decoded.height()) <= 800,
        "{}x{}",
        decoded.width(),
        decoded.height()
    );
    assert_eq!(
        e.svc.image_sha(e.h, r.id).await.unwrap().as_deref(),
        Some(img.sha256.as_str())
    );
    assert!(e.svc.get_recipe(e.h, None, r.id).await.unwrap().has_image);
    // Not an image, and too large.
    assert!(matches!(
        e.svc.put_image(e.h, r.id, b"<html>".to_vec()).await,
        Err(MealsError::Image(_))
    ));
    assert!(matches!(
        e.svc
            .put_image(
                e.h,
                r.id,
                [0x89, b'P', b'N', b'G', 0x0D, 0x0A, 0x1A, 0x0A, 0, 0].repeat(600_000)
            )
            .await,
        Err(MealsError::Image(_))
    ));
    assert!(matches!(
        e.svc.put_image(e.h, RecipeId::new(), make_png(8, 8)).await,
        Err(MealsError::NotFound)
    ));
    e.svc.delete_image(e.h, r.id).await.unwrap();
    assert!(e.svc.get_image(e.h, r.id).await.unwrap().is_none());
}

const PAGE: &str = r#"<html><head><title>Lemon Pasta</title>
<script type="application/ld+json">{"@context":"https://schema.org","@type":"Recipe","name":"Lemon Pasta",
"recipeYield":"4 servings","recipeIngredient":["200 g spaghetti","1 lemon","2 tbsp olive oil"],
"recipeInstructions":["Boil the pasta.","Toss with lemon."],"image":"http://127.0.0.1/pic.png"}</script>
</head><body></body></html>"#;

const MICRODATA: &str = r#"<html><body><div itemscope itemtype="https://schema.org/Recipe">
<h1 itemprop="name">Bean Bowl</h1><span itemprop="recipeIngredient">1 can black beans</span>
<span itemprop="recipeIngredient">1 cup rice</span><div itemprop="recipeInstructions">Heat it.</div></div></body></html>"#;

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_from_a_page_follows_the_private_host_setting(pool: PgPool) {
    let (url, state) = stub().await;
    state.lock().unwrap().page = PAGE.into();
    let open = env_with(
        pool.clone(),
        Opts {
            allow_private: true,
            ..Default::default()
        },
    )
    .await;
    let draft = open
        .svc
        .import(
            open.h,
            RecipeImportReq {
                url: Some(format!("{url}/recipe")),
                text: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (draft.name.as_str(), draft.origin, draft.ingredients.len()),
        ("Lemon Pasta", RecipeOrigin::Jsonld, 3)
    );
    assert_eq!(draft.ingredients[0].quantity, Some(200.0));
    state.lock().unwrap().page = MICRODATA.into();
    let draft = open
        .svc
        .import(
            open.h,
            RecipeImportReq {
                url: Some(format!("{url}/recipe")),
                text: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (draft.name.as_str(), draft.origin),
        ("Bean Bowl", RecipeOrigin::Microdata)
    );
    // Nothing was stored.
    assert!(
        open.svc
            .list_recipes(open.h, None, RecipeListQuery::default())
            .await
            .unwrap()
            .items
            .is_empty()
    );
    // A page without data and no LLM.
    state.lock().unwrap().page = "<html><body>Hello</body></html>".into();
    let none = open
        .svc
        .import(
            open.h,
            RecipeImportReq {
                url: Some(format!("{url}/recipe")),
                text: None,
            },
        )
        .await;
    assert!(matches!(none, Err(MealsError::LlmUnavailable)));

    // With private hosts off (the default) the stub's loopback address is refused.
    let closed = env(pool).await;
    let refused = closed
        .svc
        .import(
            closed.h,
            RecipeImportReq {
                url: Some(format!("{url}/recipe")),
                text: None,
            },
        )
        .await;
    assert!(matches!(refused, Err(MealsError::FetchFailed(_))));
    for bad in [
        RecipeImportReq {
            url: None,
            text: None,
        },
        RecipeImportReq {
            url: Some("x".into()),
            text: Some("y".into()),
        },
        RecipeImportReq {
            url: Some("file:///etc/passwd".into()),
            text: None,
        },
    ] {
        assert!(matches!(
            closed.svc.import(closed.h, bad).await,
            Err(MealsError::Invalid { .. })
        ));
    }
}

#[sqlx::test(migrations = "../fa-store/migrations")]
async fn import_text_and_unstructured_pages_use_the_llm(pool: PgPool) {
    let (url, state) = stub().await;
    let e = env_with(
        pool,
        Opts {
            llm_url: Some(url.clone()),
            allow_private: true,
            ..Default::default()
        },
    )
    .await;
    state.lock().unwrap().reply = json!({
        "name": "Pasted Soup", "yield_text": "", "servings": 4, "prep_minutes": 5, "cook_minutes": 0,
        "total_minutes": 0, "ingredients": ["2 carrots", "1 onion"], "steps": ["Simmer."], "tags": []
    });
    let draft = e
        .svc
        .import(
            e.h,
            RecipeImportReq {
                url: None,
                text: Some("Soup: carrots and onion, simmer".into()),
            },
        )
        .await
        .unwrap();
    assert_eq!(
        (draft.name.as_str(), draft.origin, draft.servings),
        ("Pasted Soup", RecipeOrigin::Llm, Some(4))
    );
    // An unstructured page goes through the model too, and the call is recorded.
    state.lock().unwrap().page = "<html><body><p>Mix flour and water.</p></body></html>".into();
    let page = e
        .svc
        .import(
            e.h,
            RecipeImportReq {
                url: Some(format!("{url}/recipe")),
                text: None,
            },
        )
        .await
        .unwrap();
    assert_eq!(page.origin, RecipeOrigin::Llm);
    assert_eq!(
        page.source_url.as_deref(),
        Some(format!("{url}/recipe").as_str())
    );
    let mut tx = e.store.tx_for(e.h).await.unwrap();
    let n: i64 =
        sqlx::query_scalar("SELECT count(*) FROM llm_call WHERE purpose = 'recipe.extract' AND ok")
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(n, 2);
    drop(tx);
    // The model finds no recipe.
    state.lock().unwrap().reply = json!({"name":"","yield_text":"","servings":0,"prep_minutes":0,"cook_minutes":0,"total_minutes":0,"ingredients":[],"steps":[],"tags":[]});
    let none = e
        .svc
        .import(
            e.h,
            RecipeImportReq {
                url: None,
                text: Some("hello".into()),
            },
        )
        .await;
    assert!(matches!(none, Err(MealsError::NoRecipe)));
    // Without an endpoint, text cannot be imported.
    let plain = env(e.pool.clone()).await;
    assert!(matches!(
        plain
            .svc
            .import(
                plain.h,
                RecipeImportReq {
                    url: None,
                    text: Some("x".into())
                }
            )
            .await,
        Err(MealsError::LlmUnavailable)
    ));
}
