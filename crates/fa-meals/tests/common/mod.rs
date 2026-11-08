//! Shared fixtures: a household with an adult and a child, a fake clock and an axum stub that
//! plays the LLM endpoint and a small website. Fictional data only.
#![allow(dead_code)]

use std::sync::{Arc, Mutex};

use axum::{
    Json, Router,
    extract::State,
    http::header,
    response::IntoResponse,
    routing::{get, post},
};
use chrono::{DateTime, NaiveDate, TimeZone, Utc};
use fa_domain::*;
use fa_meals::{ActorRef, MealsService};
use fa_store::*;
use serde_json::{Value, json};
use sqlx::PgPool;

pub fn d(day: u32) -> NaiveDate {
    NaiveDate::from_ymd_opt(2026, 11, day).unwrap()
}

/// Monday 2026-11-09, 10:00 in New York.
pub fn base_time() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 9, 15, 0, 0).unwrap()
}

pub fn member(h: HouseholdId, name: &str, role: Role) -> Member {
    Member {
        id: MemberId::new(),
        household_id: h,
        display_name: name.into(),
        role,
        avatar: None,
        color: None,
        birthdate: None,
        grade: None,
        interests: vec![],
        hobbies: vec![],
        responsibilities: vec![],
        aliases: Vec::new(),
        gender: None,
        relevance_tags: Vec::new(),
        pin_hash: None,
        archived_at: None,
        created_at: Utc::now(),
        updated_at: Utc::now(),
    }
}

#[derive(Default)]
pub struct StubState {
    pub reply: Value,
    pub requests: Vec<Value>,
    pub png: Vec<u8>,
    pub page: String,
}
pub type Stub = Arc<Mutex<StubState>>;

async fn chat(State(s): State<Stub>, Json(body): Json<Value>) -> Json<Value> {
    let mut g = s.lock().unwrap();
    g.requests.push(body);
    Json(json!({
        "choices": [{"message": {"content": g.reply.to_string()}}],
        "usage": {"prompt_tokens": 11, "completion_tokens": 7}
    }))
}

async fn png(State(s): State<Stub>) -> impl IntoResponse {
    let b = s.lock().unwrap().png.clone();
    ([(header::CONTENT_TYPE, "image/png")], b)
}

async fn page(State(s): State<Stub>) -> impl IntoResponse {
    let b = s.lock().unwrap().page.clone();
    ([(header::CONTENT_TYPE, "text/html")], b)
}

/// Returns the base URL (`http://127.0.0.1:port`) and the shared state.
pub async fn stub() -> (String, Stub) {
    let state: Stub = Arc::new(Mutex::new(StubState::default()));
    let app = Router::new()
        .route("/v1/chat/completions", post(chat))
        .route(
            "/v1/models",
            get(|| async { Json(json!({"data": [{"id": "test-model"}]})) }),
        )
        .route("/img.png", get(png))
        .route("/recipe", get(page))
        .with_state(state.clone());
    let l = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", l.local_addr().unwrap());
    tokio::spawn(async move { axum::serve(l, app).await.unwrap() });
    (url, state)
}

pub struct Env {
    pub pool: PgPool,
    pub store: Store,
    pub svc: MealsService,
    pub h: HouseholdId,
    pub adult: MemberId,
    pub kid: MemberId,
    pub actor: ActorRef,
    pub clock: Arc<Mutex<DateTime<Utc>>>,
}

pub struct Opts {
    pub llm_url: Option<String>,
    pub allow_private: bool,
    pub seed: bool,
}

impl Default for Opts {
    fn default() -> Self {
        Opts {
            llm_url: None,
            allow_private: false,
            seed: true,
        }
    }
}

pub async fn env(pool: PgPool) -> Env {
    env_with(pool, Opts::default()).await
}

pub async fn env_with(pool: PgPool, o: Opts) -> Env {
    let store = Store::from_pool(pool.clone());
    let h = HouseholdId::new();
    let adult = member(h, "Pat", Role::Adult);
    let kid = member(h, "Sam", Role::Child);
    let mut tx = store.tx_system().await.unwrap();
    sqlx::query("INSERT INTO household (id, name, tz) VALUES ($1, 'Test', 'America/New_York')")
        .bind(h.as_uuid())
        .execute(&mut *tx)
        .await
        .unwrap();
    MemberRepo::insert(&mut tx, &adult, None, None)
        .await
        .unwrap();
    MemberRepo::insert(&mut tx, &kid, None, None).await.unwrap();
    tx.commit().await.unwrap();
    let mut tx = store.tx_for(h).await.unwrap();
    if o.seed {
        fa_meals::ensure_defaults(&mut tx).await.unwrap();
    }
    if let Some(url) = &o.llm_url {
        LlmEndpointRepo::upsert(
            &mut tx,
            h,
            &LlmEndpointInput {
                base_url: format!("{url}/v1"),
                model: "test-model".into(),
                dialect: LlmDialect::OpenaiChat,
                timeout_ms: 5000,
                enabled: true,
            },
        )
        .await
        .unwrap();
    }
    tx.commit().await.unwrap();
    let llm = o.llm_url.as_ref().map(|_| {
        Arc::new(fa_llm::Provider::new(
            store.clone(),
            Arc::new(MasterKey::from_bytes(&[7u8; 32], 1).unwrap()),
            fa_llm::default_http().unwrap(),
        ))
    });
    let clock = Arc::new(Mutex::new(base_time()));
    let c = clock.clone();
    let svc = MealsService::new(store.clone(), llm, o.allow_private)
        .with_clock(Arc::new(move || *c.lock().unwrap()));
    Env {
        pool,
        store,
        svc,
        h,
        adult: adult.id,
        kid: kid.id,
        actor: ActorRef {
            member: Some(adult.id),
            device: None,
        },
        clock,
    }
}

impl Env {
    pub fn at(&self, t: DateTime<Utc>) {
        *self.clock.lock().unwrap() = t;
    }

    pub async fn slot(&self, name: &str) -> MealSlotId {
        self.svc
            .list_slots(self.h)
            .await
            .unwrap()
            .into_iter()
            .find(|s| s.name == name)
            .unwrap()
            .id
            .unwrap()
    }

    pub async fn dinner(&self) -> MealSlotId {
        self.slot("Dinner").await
    }

    pub async fn recipe(&self, title: &str, servings: Option<u32>, lines: &[&str]) -> RecipeView {
        self.svc
            .create_recipe(
                self.h,
                self.actor,
                RecipeInput {
                    title: title.into(),
                    source_url: None,
                    servings,
                    yield_text: None,
                    prep_minutes: None,
                    cook_minutes: None,
                    total_minutes: None,
                    import_method: ImportMethod::Manual,
                    tags: vec![],
                    note: None,
                    image_url: None,
                    lines: lines.iter().map(|s| s.to_string()).collect(),
                    steps: vec!["Cook it.".into()],
                },
            )
            .await
            .unwrap()
    }

    pub async fn plan(
        &self,
        date: NaiveDate,
        slot: MealSlotId,
        recipe: &RecipeView,
        servings: Option<u32>,
    ) -> MealPlanEntryView {
        self.svc
            .plan_add(
                self.h,
                self.actor,
                MealPlanEntryInput {
                    date,
                    slot_id: slot,
                    recipe_id: Some(recipe.id),
                    free_text: None,
                    servings,
                },
            )
            .await
            .unwrap()
            .entries
            .remove(0)
    }

    pub async fn free(&self, date: NaiveDate, slot: MealSlotId, text: &str) -> MealPlanEntryView {
        self.svc
            .plan_add(
                self.h,
                self.actor,
                MealPlanEntryInput {
                    date,
                    slot_id: slot,
                    recipe_id: None,
                    free_text: Some(text.into()),
                    servings: None,
                },
            )
            .await
            .unwrap()
            .entries
            .remove(0)
    }

    pub async fn staple(&self, name: &str) {
        let ing = self
            .svc
            .list_ingredients(self.h, None)
            .await
            .unwrap()
            .into_iter()
            .find(|i| i.canonical_name == name)
            .unwrap();
        self.svc
            .set_ingredient_staple(self.h, ing.id, true)
            .await
            .unwrap();
    }

    pub async fn generate_range(&self, from: u32, to: u32) -> GenerateResult {
        self.svc
            .generate(
                self.h,
                self.actor,
                GenerateReq {
                    entry_ids: None,
                    from: Some(d(from)),
                    to: Some(d(to)),
                },
            )
            .await
            .unwrap()
    }

    pub async fn items(&self) -> Vec<GroceryItemView> {
        self.svc.grocery_view(self.h).await.unwrap().items
    }

    pub async fn item(&self, text: &str) -> Option<GroceryItemView> {
        self.items().await.into_iter().find(|i| i.text == text)
    }

    pub async fn add_item(&self, text: &str) -> GroceryItemView {
        self.svc
            .grocery_add(
                self.h,
                self.actor,
                GroceryAddReq {
                    id: None,
                    text: text.into(),
                    qty: None,
                    unit: None,
                    store_id: None,
                },
            )
            .await
            .unwrap()
            .0
    }
}

/// A gradient picture of the given size, as PNG bytes.
pub fn make_png(w: u32, h: u32) -> Vec<u8> {
    let img = image::RgbImage::from_fn(w, h, |x, y| {
        image::Rgb([(x % 256) as u8, (y % 256) as u8, 128])
    });
    let mut out = Vec::new();
    image::DynamicImage::ImageRgb8(img)
        .write_to(&mut std::io::Cursor::new(&mut out), image::ImageFormat::Png)
        .unwrap();
    out
}

pub fn pp() -> MealPlanEntryPatch {
    MealPlanEntryPatch {
        date: None,
        slot_id: None,
        recipe_id: None,
        free_text: None,
        servings: None,
    }
}

pub fn gp() -> GroceryItemPatch {
    GroceryItemPatch {
        text: None,
        qty: None,
        unit: None,
        store_id: None,
    }
}

pub fn sp() -> MealsSettingsPatch {
    MealsSettingsPatch {
        grocery_archive_hours: None,
        ingredient_llm: None,
        trigram_threshold: None,
        descriptors: None,
        meal_plan_horizon_weeks: None,
    }
}
