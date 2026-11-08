//! M5: meals schema and repos (fictional data only).

use chrono::{Duration, NaiveDate, NaiveTime, Utc};
use fa_domain::*;
use fa_store::*;
use sqlx::PgPool;

async fn seed(store: &Store, name: &str) -> HouseholdId {
    let id = HouseholdId::new();
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, $2, 'America/New_York')")
        .bind(id.as_uuid())
        .bind(name)
        .execute(&mut *tx)
        .await
        .unwrap();
    tx.commit().await.unwrap();
    id
}

fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
}

fn fields(title: &str) -> RecipeFields {
    RecipeFields {
        title: title.into(),
        servings: Some(4),
        tags: vec!["quick".into()],
        ..Default::default()
    }
}

async fn recipe(conn: &mut sqlx::PgConnection, h: HouseholdId, title: &str) -> RecipeRow {
    RecipeRepo::insert(
        conn,
        RecipeId::new(),
        h,
        &fields(title),
        ImportMethod::Manual,
        None,
    )
    .await
    .unwrap()
}

async fn slot(conn: &mut sqlx::PgConnection, h: HouseholdId, name: &str, ord: i32) -> MealSlotRow {
    MealSlotRepo::insert(
        conn,
        h,
        name,
        ord,
        NaiveTime::from_hms_opt(18, 0, 0).unwrap(),
        true,
    )
    .await
    .unwrap()
}

fn free(date: NaiveDate, slot: MealSlotId, text: &str) -> NewEntry {
    NewEntry {
        date,
        slot_id: slot,
        content: EntryContent::FreeText(text.into()),
        servings: None,
        created_by: None,
    }
}

fn new_ing(name: &str, aliases: &[&str]) -> NewIngredient {
    NewIngredient {
        canonical_name: name.into(),
        aliases: aliases.iter().map(|s| s.to_string()).collect(),
        is_staple: false,
        aisle_id: None,
        default_store_id: None,
        default_unit: None,
        origin: IngredientOrigin::User,
    }
}

fn new_item(text: &str) -> NewGroceryItem {
    NewGroceryItem {
        id: None,
        ingredient_id: None,
        text: text.into(),
        qty: None,
        unit: None,
        qty2: None,
        unit2: None,
        store_id: None,
        aisle_id: None,
        section: GrocerySection::List,
        manual: true,
        added_by: None,
        added_via_device: None,
    }
}

fn line(raw: &str) -> NewLine {
    NewLine {
        raw_text: raw.into(),
        qty: Some(1.0),
        unit: None,
        item: Some(raw.into()),
        note: None,
        ingredient_id: None,
        match_state: MatchState::Unmatched,
        proposal: None,
    }
}

const TABLES: [&str; 18] = [
    "aisle",
    "grocery_store",
    "grocery_store_aisle",
    "recipe",
    "recipe_image",
    "ingredient",
    "recipe_ingredient",
    "recipe_step",
    "recipe_rating",
    "meal_slot",
    "meal_plan_entry",
    "meal_template",
    "meal_template_cell",
    "grocery_list",
    "grocery_item",
    "grocery_item_source",
    "meals_change",
    "meals_settings",
];

async fn table_count(pool: &PgPool) -> i64 {
    sqlx::query_scalar(
        "SELECT count(*) FROM pg_tables WHERE schemaname = 'public' AND tablename = ANY ($1)",
    )
    .bind(TABLES.iter().map(|t| t.to_string()).collect::<Vec<_>>())
    .fetch_one(pool)
    .await
    .unwrap()
}

#[sqlx::test(migrations = "./migrations")]
async fn migration_applies_and_reverts(pool: PgPool) {
    assert_eq!(table_count(&pool).await, 18);
    let rls: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM pg_class WHERE relname = ANY ($1) AND relrowsecurity AND relforcerowsecurity",
    )
    .bind(TABLES.iter().map(|t| t.to_string()).collect::<Vec<_>>())
    .fetch_one(&pool)
    .await
    .unwrap();
    assert_eq!(rls, 18);
    sqlx::raw_sql(include_str!(
        "../migrations/20261108000001_m5_meals.down.sql"
    ))
    .execute(&pool)
    .await
    .unwrap();
    assert_eq!(table_count(&pool).await, 0);
    let ext: i64 =
        sqlx::query_scalar("SELECT count(*) FROM pg_extension WHERE extname = 'pg_trgm'")
            .fetch_one(&pool)
            .await
            .unwrap();
    assert_eq!(ext, 1, "the down migration keeps the extension");
    sqlx::raw_sql(include_str!("../migrations/20261108000001_m5_meals.up.sql"))
        .execute(&pool)
        .await
        .unwrap();
    assert_eq!(table_count(&pool).await, 18);
}

#[sqlx::test(migrations = "./migrations")]
async fn rls_hides_other_households(pool: PgPool) {
    let store = Store::from_pool(pool);
    let (a, b) = (seed(&store, "A").await, seed(&store, "B").await);
    let mut tx = store.tx_for(a).await.unwrap();
    let r = recipe(&mut tx, a, "Lentil soup").await;
    let s = slot(&mut tx, a, "Dinner", 0).await;
    MealPlanRepo::insert(&mut tx, a, &free(d(9), s.id, "Leftovers"))
        .await
        .unwrap();
    let list = GroceryRepo::list_id(&mut tx, a).await.unwrap();
    GroceryRepo::insert_item(&mut tx, a, list, &new_item("milk"))
        .await
        .unwrap();
    tx.commit().await.unwrap();

    let mut tx = store.tx_for(b).await.unwrap();
    assert!(RecipeRepo::get(&mut tx, r.id).await.unwrap().is_none());
    assert!(
        MealPlanRepo::range(&mut tx, d(1), d(30))
            .await
            .unwrap()
            .is_empty()
    );
    assert!(
        GroceryRepo::items_open(&mut tx, list)
            .await
            .unwrap()
            .is_empty()
    );
    assert!(MealSlotRepo::list(&mut tx).await.unwrap().is_empty());
    // A row of the other household cannot be written either.
    let err =
        sqlx::query("INSERT INTO aisle (id, household_id, name, ord) VALUES ($1, $2, 'x', 0)")
            .bind(uuid::Uuid::now_v7())
            .bind(a.as_uuid())
            .execute(&mut *tx)
            .await;
    assert!(err.is_err());
}

#[sqlx::test(migrations = "./migrations")]
async fn several_entries_per_slot_keep_their_order(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let dinner = slot(&mut tx, h, "Dinner", 0).await;
    let lunch = slot(&mut tx, h, "Lunch", 1).await;
    let mut ids = Vec::new();
    for t in ["Soup", "Bread", "Salad"] {
        ids.push(
            MealPlanRepo::insert(&mut tx, h, &free(d(9), dinner.id, t))
                .await
                .unwrap(),
        );
    }
    let other = MealPlanRepo::insert(&mut tx, h, &free(d(9), lunch.id, "Wrap"))
        .await
        .unwrap();
    assert_eq!(ids.iter().map(|e| e.ord).collect::<Vec<_>>(), vec![0, 1, 2]);
    assert_eq!(other.ord, 0, "ord counts per date and slot");
    let at = MealPlanRepo::entries_at(&mut tx, d(9), dinner.id)
        .await
        .unwrap();
    assert_eq!(
        at.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
        vec!["Soup", "Bread", "Salad"]
    );
    assert_eq!(
        MealPlanRepo::next_ord(&mut tx, d(9), dinner.id)
            .await
            .unwrap(),
        3
    );
    // Moving an entry to another slot appends there.
    let v = MealPlanRepo::update(
        &mut tx,
        ids[0].id,
        &EntryPatch {
            slot_id: Some(lunch.id),
            ..Default::default()
        },
        1,
    )
    .await
    .unwrap();
    assert_eq!(v, 2);
    let moved = MealPlanRepo::get(&mut tx, ids[0].id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((moved.slot_id, moved.ord), (lunch.id, 1));
    // The range is ordered by date, slot order, then ord (Dinner is slot 0).
    let all = MealPlanRepo::range(&mut tx, d(9), d(9)).await.unwrap();
    assert_eq!(
        all.iter().map(|e| e.title.as_str()).collect::<Vec<_>>(),
        vec!["Bread", "Salad", "Wrap", "Soup"]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn swap_exchanges_position(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let dinner = slot(&mut tx, h, "Dinner", 0).await;
    let lunch = slot(&mut tx, h, "Lunch", 1).await;
    let a = MealPlanRepo::insert(&mut tx, h, &free(d(9), dinner.id, "Soup"))
        .await
        .unwrap();
    let a2 = MealPlanRepo::insert(&mut tx, h, &free(d(9), dinner.id, "Bread"))
        .await
        .unwrap();
    let b = MealPlanRepo::insert(&mut tx, h, &free(d(10), lunch.id, "Wrap"))
        .await
        .unwrap();
    MealPlanRepo::swap(&mut tx, a2.id, b.id).await.unwrap();
    let a2n = MealPlanRepo::get(&mut tx, a2.id).await.unwrap().unwrap();
    let bn = MealPlanRepo::get(&mut tx, b.id).await.unwrap().unwrap();
    assert_eq!((a2n.date, a2n.slot_id, a2n.ord), (b.date, b.slot_id, b.ord));
    assert_eq!((bn.date, bn.slot_id, bn.ord), (a2.date, a2.slot_id, a2.ord));
    assert_eq!((a2n.version, bn.version), (2, 2));
    let an = MealPlanRepo::get(&mut tx, a.id).await.unwrap().unwrap();
    assert_eq!(
        (an.date, an.slot_id, an.ord, an.version),
        (a.date, a.slot_id, 0, 1)
    );
    assert!(matches!(
        MealPlanRepo::swap(&mut tx, a.id, MealPlanEntryId::new()).await,
        Err(StoreError::NotFound)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn store_aisle_order_falls_back_and_honours_hidden(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let aisles = AisleRepo::replace(
        &mut tx,
        h,
        &[
            (None, "Produce".into()),
            (None, "Dairy".into()),
            (None, "Bakery".into()),
        ],
    )
    .await
    .unwrap();
    assert_eq!(
        aisles.iter().map(|a| a.ord).collect::<Vec<_>>(),
        vec![0, 1, 2]
    );
    let stores = GroceryStoreRepo::replace(&mut tx, h, &[(None, "Corner Market".into())])
        .await
        .unwrap();
    let st = stores[0].id;
    // A new store copies the household order.
    let rows = GroceryStoreRepo::aisles_for(&mut tx, st).await.unwrap();
    assert_eq!(
        rows.iter().map(|r| r.aisle_id).collect::<Vec<_>>(),
        aisles.iter().map(|a| a.id).collect::<Vec<_>>()
    );
    // Reorder and hide for this store only; Bakery's row is missing, so it falls back to ord 2.
    GroceryStoreRepo::set_aisles(&mut tx, st, &[(aisles[1].id, false), (aisles[0].id, true)])
        .await
        .unwrap();
    let rows = GroceryStoreRepo::aisles_for(&mut tx, st).await.unwrap();
    assert_eq!(
        rows.iter()
            .map(|r| (r.aisle_id, r.ord, r.hidden))
            .collect::<Vec<_>>(),
        vec![
            (aisles[1].id, 0, false),
            (aisles[0].id, 1, true),
            (aisles[2].id, 2, false)
        ]
    );
    // A store that was never configured shows the household order, nothing hidden.
    let other = GroceryStoreRepo::replace(
        &mut tx,
        h,
        &[(Some(st), "Corner Market".into()), (None, "Big Box".into())],
    )
    .await
    .unwrap();
    assert_eq!(other.len(), 2);
    let plain = GroceryStoreRepo::aisles_for(&mut tx, other[1].id)
        .await
        .unwrap();
    assert!(plain.iter().all(|r| !r.hidden));
    // Renaming and swapping names in one replace does not trip the unique index.
    let swapped = AisleRepo::replace(
        &mut tx,
        h,
        &[
            (Some(aisles[1].id), "Produce".into()),
            (Some(aisles[0].id), "Dairy".into()),
        ],
    )
    .await
    .unwrap();
    assert_eq!(swapped.len(), 2);
    assert_eq!(swapped[0].name, "Produce");
    // The removed aisle's per-store row went with it.
    let rows = GroceryStoreRepo::aisles_for(&mut tx, st).await.unwrap();
    assert_eq!(rows.len(), 2);
}

#[sqlx::test(migrations = "./migrations")]
async fn insert_item_with_client_id_is_idempotent(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let list = GroceryRepo::list_id(&mut tx, h).await.unwrap();
    assert_eq!(GroceryRepo::list_id(&mut tx, h).await.unwrap(), list);
    let id = GroceryItemId::new();
    let mut n = new_item("oat milk");
    n.id = Some(id);
    let (first, created) = GroceryRepo::insert_item(&mut tx, h, list, &n)
        .await
        .unwrap();
    assert!(created);
    n.text = "different text on replay".into();
    let (again, created) = GroceryRepo::insert_item(&mut tx, h, list, &n)
        .await
        .unwrap();
    assert!(!created);
    assert_eq!(again.text, "oat milk");
    assert_eq!(again.id, first.id);
    assert_eq!(
        GroceryRepo::items_open(&mut tx, list).await.unwrap().len(),
        1
    );
    // Check and uncheck are idempotent and ignore versions.
    let c1 = GroceryRepo::check(&mut tx, id, None).await.unwrap();
    let c2 = GroceryRepo::check(&mut tx, id, None).await.unwrap();
    assert!(c1.checked_at.is_some());
    assert_eq!((c1.version, c2.version), (2, 2));
    let u = GroceryRepo::uncheck(&mut tx, id).await.unwrap();
    assert!(u.checked_at.is_none());
    assert_eq!(u.version, 3);
    // Staples section round trip.
    let s = GroceryRepo::set_section(&mut tx, id, GrocerySection::Staples)
        .await
        .unwrap();
    assert_eq!(s.section, GrocerySection::Staples);
}

#[sqlx::test(migrations = "./migrations")]
async fn stale_versions_conflict(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let r = recipe(&mut tx, h, "Pancakes").await;
    let mut f = r.fields.clone();
    f.title = "Fluffy pancakes".into();
    assert_eq!(RecipeRepo::update(&mut tx, r.id, &f, 1).await.unwrap(), 2);
    assert!(matches!(
        RecipeRepo::update(&mut tx, r.id, &f, 1).await,
        Err(StoreError::Conflict)
    ));
    assert!(matches!(
        RecipeRepo::update(&mut tx, RecipeId::new(), &f, 1).await,
        Err(StoreError::NotFound)
    ));

    let s = slot(&mut tx, h, "Dinner", 0).await;
    let e = MealPlanRepo::insert(&mut tx, h, &free(d(9), s.id, "Soup"))
        .await
        .unwrap();
    let patch = EntryPatch {
        servings: Some(Some(6)),
        ..Default::default()
    };
    assert_eq!(
        MealPlanRepo::update(&mut tx, e.id, &patch, 1)
            .await
            .unwrap(),
        2
    );
    assert!(matches!(
        MealPlanRepo::update(&mut tx, e.id, &patch, 1).await,
        Err(StoreError::Conflict)
    ));
    let list = GroceryRepo::list_id(&mut tx, h).await.unwrap();
    let (item, _) = GroceryRepo::insert_item(&mut tx, h, list, &new_item("rice"))
        .await
        .unwrap();
    let edit = GroceryItemEdit {
        text: Some("brown rice".into()),
        ..Default::default()
    };
    assert_eq!(
        GroceryRepo::update_item(&mut tx, item.id, &edit, 1)
            .await
            .unwrap(),
        2
    );
    assert!(matches!(
        GroceryRepo::update_item(&mut tx, item.id, &edit, 1).await,
        Err(StoreError::Conflict)
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn nearest_ranks_the_typo_first_and_aliases_match(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let n = IngredientRepo::insert_many(
        &mut tx,
        h,
        &[
            new_ing("tomato", &["roma tomato", "plum tomato"]),
            new_ing("tamarind", &[]),
            new_ing("potato", &["spud"]),
            new_ing("basil", &[]),
        ],
    )
    .await
    .unwrap();
    assert_eq!(n, 4);
    // A repeat inserts nothing.
    assert_eq!(
        IngredientRepo::insert_many(&mut tx, h, &[new_ing("tomato", &[])])
            .await
            .unwrap(),
        0
    );
    let near = IngredientRepo::nearest(&mut tx, "tomatoe", 3)
        .await
        .unwrap();
    assert_eq!(near.len(), 3);
    assert_eq!(near[0].1, "tomato");
    assert!(near[0].2 > near[1].2, "{near:?}");
    assert!(near[0].2 > 0.45);

    assert_eq!(
        IngredientRepo::match_exact(&mut tx, "Tomato")
            .await
            .unwrap()
            .unwrap()
            .canonical_name,
        "tomato"
    );
    assert!(
        IngredientRepo::match_exact(&mut tx, "roma tomato")
            .await
            .unwrap()
            .is_none()
    );
    let by_alias = IngredientRepo::match_alias(&mut tx, "Roma Tomato")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(by_alias.canonical_name, "tomato");
    assert!(
        IngredientRepo::match_alias(&mut tx, "tomato")
            .await
            .unwrap()
            .is_none()
    );

    // add_alias skips the canonical name and repeats (any case).
    assert!(
        IngredientRepo::add_alias(&mut tx, by_alias.id, "cherry tomato")
            .await
            .unwrap()
    );
    assert!(
        !IngredientRepo::add_alias(&mut tx, by_alias.id, "Cherry Tomato")
            .await
            .unwrap()
    );
    assert!(
        !IngredientRepo::add_alias(&mut tx, by_alias.id, "TOMATO")
            .await
            .unwrap()
    );
    let found = IngredientRepo::search(&mut tx, "spu", 0.45, 10)
        .await
        .unwrap();
    assert_eq!(found.len(), 1);
    assert_eq!(found[0].canonical_name, "potato");
}

#[sqlx::test(migrations = "./migrations")]
async fn merge_repoints_lines_and_items(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let keep = IngredientRepo::insert(&mut tx, h, &new_ing("scallion", &["spring onion"]))
        .await
        .unwrap();
    let gone = IngredientRepo::insert(
        &mut tx,
        h,
        &new_ing("green onion", &["Spring Onion", "salad onion"]),
    )
    .await
    .unwrap();
    let r = recipe(&mut tx, h, "Noodles").await;
    let mut l = line("2 green onions");
    l.ingredient_id = Some(gone.id);
    l.match_state = MatchState::Matched;
    RecipeLineRepo::replace_for_recipe(&mut tx, r.id, &[l])
        .await
        .unwrap();
    let list = GroceryRepo::list_id(&mut tx, h).await.unwrap();
    let mut it = new_item("green onion");
    it.ingredient_id = Some(gone.id);
    let (item, _) = GroceryRepo::insert_item(&mut tx, h, list, &it)
        .await
        .unwrap();

    IngredientRepo::merge(&mut tx, gone.id, keep.id)
        .await
        .unwrap();

    assert!(
        IngredientRepo::get(&mut tx, gone.id)
            .await
            .unwrap()
            .is_none()
    );
    let lines = RecipeLineRepo::for_recipe(&mut tx, r.id).await.unwrap();
    assert_eq!(lines[0].ingredient_id, Some(keep.id));
    assert_eq!(lines[0].ingredient_name.as_deref(), Some("scallion"));
    assert_eq!(lines[0].match_state, MatchState::Matched);
    let item = GroceryRepo::get_item(&mut tx, item.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(item.ingredient_id, Some(keep.id));
    let merged = IngredientRepo::get(&mut tx, keep.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        merged.aliases,
        vec![
            "spring onion".to_string(),
            "green onion".into(),
            "salad onion".into()
        ]
    );
    assert!(matches!(
        IngredientRepo::merge(&mut tx, keep.id, keep.id).await,
        Err(StoreError::Corrupt(_))
    ));
}

#[sqlx::test(migrations = "./migrations")]
async fn archive_checked_before_only_archives_old_checked_items(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let list = GroceryRepo::list_id(&mut tx, h).await.unwrap();
    let mut ids = Vec::new();
    for t in ["old", "fresh", "open"] {
        ids.push(
            GroceryRepo::insert_item(&mut tx, h, list, &new_item(t))
                .await
                .unwrap()
                .0
                .id,
        );
    }
    GroceryRepo::check(&mut tx, ids[0], None).await.unwrap();
    GroceryRepo::check(&mut tx, ids[1], None).await.unwrap();
    sqlx::query("UPDATE grocery_item SET checked_at = now() - interval '30 hours' WHERE id = $1")
        .bind(ids[0].as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    let cutoff = Utc::now() - Duration::hours(24);
    assert_eq!(
        GroceryRepo::archive_checked_before(&mut tx, cutoff)
            .await
            .unwrap(),
        1
    );
    assert_eq!(
        GroceryRepo::archive_checked_before(&mut tx, cutoff)
            .await
            .unwrap(),
        0
    );
    let open = GroceryRepo::items_open(&mut tx, list).await.unwrap();
    assert_eq!(
        open.iter().map(|i| i.text.as_str()).collect::<Vec<_>>(),
        vec!["fresh", "open"]
    );
    let old = GroceryRepo::get_item(&mut tx, ids[0])
        .await
        .unwrap()
        .unwrap();
    assert!(old.archived_at.is_some());
}

#[sqlx::test(migrations = "./migrations")]
async fn lines_keep_id_and_match_when_text_is_unchanged(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let ing = IngredientRepo::insert(&mut tx, h, &new_ing("flour", &[]))
        .await
        .unwrap();
    let r = recipe(&mut tx, h, "Bread").await;
    let first = RecipeLineRepo::replace_for_recipe(
        &mut tx,
        r.id,
        &[line("2 cups flour"), line("1 tsp salt")],
    )
    .await
    .unwrap();
    assert!(first.iter().all(|l| !l.kept));
    RecipeLineRepo::set_match(
        &mut tx,
        first[0].row.id,
        Some(ing.id),
        MatchState::Matched,
        None,
    )
    .await
    .unwrap();
    // Reorder, drop salt, add water: flour keeps its id and match.
    let second = RecipeLineRepo::replace_for_recipe(
        &mut tx,
        r.id,
        &[line("1 cup water"), line("2 cups flour")],
    )
    .await
    .unwrap();
    assert!(!second[0].kept && second[1].kept);
    assert_eq!(second[1].row.id, first[0].row.id);
    assert_eq!(second[1].row.ingredient_id, Some(ing.id));
    assert_eq!(second[1].row.match_state, MatchState::Matched);
    assert_eq!(second[1].row.ord, 1);
    let all = RecipeLineRepo::for_recipe(&mut tx, r.id).await.unwrap();
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].raw_text, "1 cup water");
    assert_eq!(all[0].match_state, MatchState::Unmatched);
    assert!(
        RecipeLineRepo::get(&mut tx, first[1].row.id)
            .await
            .unwrap()
            .is_none()
    );
    // Steps are replaced whole.
    RecipeStepRepo::replace_for_recipe(&mut tx, r.id, &["Mix".into(), "Bake".into()])
        .await
        .unwrap();
    RecipeStepRepo::replace_for_recipe(&mut tx, r.id, &["Bake".into()])
        .await
        .unwrap();
    assert_eq!(
        RecipeStepRepo::for_recipe(&mut tx, r.id).await.unwrap(),
        vec!["Bake"]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn deleting_a_planned_recipe_keeps_the_entry_as_free_text(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let r = recipe(&mut tx, h, "Shakshuka").await;
    let s = slot(&mut tx, h, "Dinner", 0).await;
    let e = MealPlanRepo::insert(
        &mut tx,
        h,
        &NewEntry {
            date: d(9),
            slot_id: s.id,
            content: EntryContent::Recipe(r.id),
            servings: Some(3),
            created_by: None,
        },
    )
    .await
    .unwrap();
    assert_eq!(e.title, "Shakshuka");
    assert_eq!(e.recipe_id, Some(r.id));
    RecipeRepo::delete(&mut tx, r.id).await.unwrap();
    let e = MealPlanRepo::get(&mut tx, e.id).await.unwrap().unwrap();
    assert_eq!(e.recipe_id, None);
    assert_eq!(e.free_text.as_deref(), Some("Shakshuka"));
    assert_eq!(e.title, "Shakshuka");
    assert!(matches!(
        RecipeRepo::delete(&mut tx, r.id).await,
        Err(StoreError::NotFound)
    ));
    // The entry can be restored exactly after a delete (undo).
    let removed = MealPlanRepo::delete_at(&mut tx, d(9), s.id).await.unwrap();
    assert_eq!(removed.len(), 1);
    MealPlanRepo::restore(&mut tx, h, &removed[0])
        .await
        .unwrap();
    assert_eq!(
        MealPlanRepo::get(&mut tx, e.id).await.unwrap().unwrap(),
        removed[0]
    );
}

#[sqlx::test(migrations = "./migrations")]
async fn library_list_searches_titles_and_ingredients_and_pages(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let a = recipe(&mut tx, h, "Apple crumble").await;
    let b = recipe(&mut tx, h, "Banana bread").await;
    let c = recipe(&mut tx, h, "Cabbage stew").await;
    let mut l = line("3 ripe bananas");
    l.item = Some("banana".into());
    RecipeLineRepo::replace_for_recipe(&mut tx, a.id, &[l])
        .await
        .unwrap();
    RecipeRepo::set_favourite(&mut tx, c.id, true)
        .await
        .unwrap();

    let q = |s: &str| RecipeQuery {
        q: Some(s.into()),
        limit: 50,
        ..Default::default()
    };
    let by_ing = RecipeRepo::list(&mut tx, &q("banana")).await.unwrap();
    assert_eq!(
        by_ing.iter().map(|r| r.title.as_str()).collect::<Vec<_>>(),
        vec!["Apple crumble", "Banana bread"]
    );
    assert_eq!(
        RecipeRepo::list(&mut tx, &q("100% real"))
            .await
            .unwrap()
            .len(),
        0
    );
    let favs = RecipeRepo::list(
        &mut tx,
        &RecipeQuery {
            favourite: Some(true),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(favs.len(), 1);
    assert_eq!(favs[0].id, c.id);
    let tagged = RecipeRepo::list(
        &mut tx,
        &RecipeQuery {
            tag: Some("quick".into()),
            limit: 50,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(tagged.len(), 3);

    // Keyset paging: two pages of two and one.
    let page1 = RecipeRepo::list(
        &mut tx,
        &RecipeQuery {
            limit: 2,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page1.len(), 2);
    let last = page1.last().unwrap();
    let page2 = RecipeRepo::list(
        &mut tx,
        &RecipeQuery {
            limit: 2,
            after: Some((last.title_key.clone(), last.id)),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(page2.iter().map(|r| r.id).collect::<Vec<_>>(), vec![c.id]);
    assert!(b.id != c.id);
    assert_eq!(RecipeRepo::all_tags(&mut tx).await.unwrap(), vec!["quick"]);
}

#[sqlx::test(migrations = "./migrations")]
async fn ratings_images_and_last_planned(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let m1 = MemberId::new();
    let m2 = MemberId::new();
    for (m, n) in [(m1, "Ada"), (m2, "Bo")] {
        sqlx::query(
            "INSERT INTO member (id, household_id, display_name, role) VALUES ($1, $2, $3, 'adult')",
        )
        .bind(m.as_uuid())
        .bind(h.as_uuid())
        .bind(n)
        .execute(&mut *tx)
        .await
        .unwrap();
    }
    let r = recipe(&mut tx, h, "Dal").await;
    RecipeRatingRepo::set(&mut tx, r.id, m1, 5).await.unwrap();
    RecipeRatingRepo::set(&mut tx, r.id, m1, 4).await.unwrap();
    RecipeRatingRepo::set(&mut tx, r.id, m2, 2).await.unwrap();
    let ratings = RecipeRatingRepo::for_recipe(&mut tx, r.id).await.unwrap();
    assert_eq!(ratings.len(), 2);
    let s = slot(&mut tx, h, "Dinner", 0).await;
    for day in [3, 9] {
        MealPlanRepo::insert(
            &mut tx,
            h,
            &NewEntry {
                date: d(day),
                slot_id: s.id,
                content: EntryContent::Recipe(r.id),
                servings: None,
                created_by: None,
            },
        )
        .await
        .unwrap();
    }
    let rows = RecipeRepo::list(
        &mut tx,
        &RecipeQuery {
            me: Some(m1),
            today: Some(d(5)),
            limit: 10,
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!(rows[0].my_rating, Some(4));
    assert_eq!(rows[0].rating_avg, Some(3.0));
    assert_eq!(rows[0].last_planned, Some(d(3)));
    assert!(!rows[0].has_image);
    RecipeRatingRepo::clear(&mut tx, r.id, m2).await.unwrap();
    assert_eq!(
        RecipeRatingRepo::for_recipe(&mut tx, r.id)
            .await
            .unwrap()
            .len(),
        1
    );

    RecipeImageRepo::put(&mut tx, r.id, "image/png", &[1, 2, 3], "abc", "upload")
        .await
        .unwrap();
    RecipeImageRepo::put(&mut tx, r.id, "image/webp", &[9; 10], "def", "import")
        .await
        .unwrap();
    let img = RecipeImageRepo::get(&mut tx, r.id).await.unwrap().unwrap();
    assert_eq!(
        (img.mime.as_str(), img.bytes.len(), img.sha256.as_str()),
        ("image/webp", 10, "def")
    );
    assert!(
        RecipeRepo::get(&mut tx, r.id)
            .await
            .unwrap()
            .unwrap()
            .has_image
    );
    assert!(RecipeImageRepo::delete(&mut tx, r.id).await.unwrap());
    assert!(!RecipeImageRepo::delete(&mut tx, r.id).await.unwrap());
}

#[sqlx::test(migrations = "./migrations")]
async fn slots_templates_and_settings(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let t = |hh, mm| NaiveTime::from_hms_opt(hh, mm, 0).unwrap();
    let SlotReplace::Done(slots) = MealSlotRepo::replace(
        &mut tx,
        h,
        &[
            SlotInput {
                id: None,
                name: "Lunch".into(),
                time: t(12, 0),
                visible: false,
            },
            SlotInput {
                id: None,
                name: "Dinner".into(),
                time: t(18, 0),
                visible: true,
            },
        ],
    )
    .await
    .unwrap() else {
        panic!("not in use yet")
    };
    let (lunch, dinner) = (slots[0].id, slots[1].id);
    let r = recipe(&mut tx, h, "Chili").await;

    let tpl = MealTemplateRepo::insert(
        &mut tx,
        h,
        &NewTemplate {
            name: "Week A".into(),
            days: 7,
            note: None,
        },
        &[
            NewTemplateCell {
                day: 1,
                slot_id: dinner,
                ord: 0,
                content: EntryContent::Recipe(r.id),
                servings: None,
            },
            NewTemplateCell {
                day: 1,
                slot_id: dinner,
                ord: 1,
                content: EntryContent::FreeText("Salad".into()),
                servings: Some(2),
            },
            NewTemplateCell {
                day: 3,
                slot_id: lunch,
                ord: 0,
                content: EntryContent::FreeText("Leftovers".into()),
                servings: None,
            },
        ],
    )
    .await
    .unwrap();
    let (row, cells) = MealTemplateRepo::get_with_cells(&mut tx, tpl.id)
        .await
        .unwrap()
        .unwrap();
    assert_eq!((row.days, row.version, cells.len()), (7, 1, 3));
    assert_eq!(cells[0].title, "Chili");
    let list = MealTemplateRepo::list(&mut tx).await.unwrap();
    assert_eq!(list[0].slot_ids, vec![lunch, dinner]);

    // A used slot cannot be removed; nothing changes.
    let res = MealSlotRepo::replace(
        &mut tx,
        h,
        &[SlotInput {
            id: Some(dinner),
            name: "Dinner".into(),
            time: t(18, 30),
            visible: true,
        }],
    )
    .await
    .unwrap();
    assert_eq!(res, SlotReplace::InUse(lunch));
    assert_eq!(MealSlotRepo::list(&mut tx).await.unwrap().len(), 2);

    // Duplicate, then edit the original: the copy is untouched.
    let copy = MealTemplateRepo::duplicate(&mut tx, tpl.id, "Week B")
        .await
        .unwrap();
    assert_ne!(copy.id, tpl.id);
    let v = MealTemplateRepo::update_with_cells(
        &mut tx,
        tpl.id,
        &NewTemplate {
            name: "Week A2".into(),
            days: 3,
            note: Some("n".into()),
        },
        &[],
        1,
    )
    .await
    .unwrap();
    assert_eq!(v, 2);
    assert!(matches!(
        MealTemplateRepo::update_with_cells(
            &mut tx,
            tpl.id,
            &NewTemplate {
                name: "x".into(),
                days: 3,
                note: None
            },
            &[],
            1
        )
        .await,
        Err(StoreError::Conflict)
    ));
    assert!(
        MealTemplateRepo::cells(&mut tx, tpl.id)
            .await
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        MealTemplateRepo::cells(&mut tx, copy.id)
            .await
            .unwrap()
            .len(),
        3
    );
    MealTemplateRepo::delete(&mut tx, tpl.id).await.unwrap();

    // Deleting the recipe removes its template cells (CASCADE) but not the free-text ones.
    RecipeRepo::delete(&mut tx, r.id).await.unwrap();
    assert_eq!(
        MealTemplateRepo::cells(&mut tx, copy.id)
            .await
            .unwrap()
            .len(),
        2
    );

    // Settings: defaults on first read, then a partial update.
    let s = MealsSettingsRepo::get_or_default(&mut tx, h).await.unwrap();
    assert_eq!((s.grocery_archive_hours, s.ingredient_llm), (24, true));
    assert!((s.trigram_threshold - 0.45).abs() < 1e-6);
    assert!(s.descriptors.is_empty());
    let s = MealsSettingsRepo::update(
        &mut tx,
        h,
        &MealsSettingsPatchRow {
            grocery_archive_hours: Some(48),
            ..Default::default()
        },
    )
    .await
    .unwrap();
    assert_eq!((s.grocery_archive_hours, s.ingredient_llm), (48, true));

    // Horizon.
    HouseholdRepo::set_meal_plan_horizon(&mut tx, h, 4)
        .await
        .unwrap();
    let weeks: i32 =
        sqlx::query_scalar("SELECT meal_plan_horizon_weeks FROM household WHERE id = $1")
            .bind(h.as_uuid())
            .fetch_one(&mut *tx)
            .await
            .unwrap();
    assert_eq!(weeks, 4);
}

#[sqlx::test(migrations = "./migrations")]
async fn changes_and_sources(pool: PgPool) {
    let store = Store::from_pool(pool);
    let h = seed(&store, "A").await;
    let mut tx = store.tx_for(h).await.unwrap();
    let s = slot(&mut tx, h, "Dinner", 0).await;
    let e = MealPlanRepo::insert(&mut tx, h, &free(d(9), s.id, "Soup"))
        .await
        .unwrap();
    let list = GroceryRepo::list_id(&mut tx, h).await.unwrap();
    let (item, _) = GroceryRepo::insert_item(&mut tx, h, list, &new_item("carrots"))
        .await
        .unwrap();
    let src = GrocerySourceRow {
        item_id: item.id,
        entry_id: e.id,
        qty: Some(300.0),
        unit: Some("g".into()),
        qty2: None,
        unit2: None,
        generated_from: d(9),
        generated_to: d(15),
    };
    GroceryRepo::upsert_source(&mut tx, h, &src).await.unwrap();
    GroceryRepo::upsert_source(
        &mut tx,
        h,
        &GrocerySourceRow {
            qty: Some(450.0),
            ..src.clone()
        },
    )
    .await
    .unwrap();
    let got = GroceryRepo::sources_for_entries(&mut tx, &[e.id])
        .await
        .unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!(got[0].qty, Some(450.0));
    let hints = GroceryRepo::hints_for_items(&mut tx, &[item.id])
        .await
        .unwrap();
    assert_eq!(
        (hints[0].date, hints[0].slot_name.as_str()),
        (d(9), "Dinner")
    );
    // Deleting the entry removes its sources (CASCADE).
    MealPlanRepo::delete(&mut tx, e.id).await.unwrap();
    assert!(
        GroceryRepo::sources_for_entries(&mut tx, &[e.id])
            .await
            .unwrap()
            .is_empty()
    );

    let ch = MealsChangeRepo::insert(
        &mut tx,
        h,
        MealsChangeKind::Generate,
        None,
        None,
        &serde_json::json!({"delete_items": [item.id]}),
        Utc::now() + Duration::minutes(10),
    )
    .await
    .unwrap();
    assert_eq!(ch.kind, MealsChangeKind::Generate);
    assert!(MealsChangeRepo::mark_undone(&mut tx, ch.id).await.unwrap());
    assert!(!MealsChangeRepo::mark_undone(&mut tx, ch.id).await.unwrap());
    assert!(
        MealsChangeRepo::get(&mut tx, ch.id)
            .await
            .unwrap()
            .unwrap()
            .undone_at
            .is_some()
    );
    assert_eq!(
        MealsChangeRepo::prune_before(&mut tx, Utc::now())
            .await
            .unwrap(),
        0
    );
    assert_eq!(
        MealsChangeRepo::prune_before(&mut tx, Utc::now() + Duration::hours(1))
            .await
            .unwrap(),
        1
    );
}
