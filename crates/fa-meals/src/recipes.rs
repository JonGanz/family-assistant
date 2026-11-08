//! Recipes: library, create, update, delete, ratings, line matching, images and import
//! (arch/Meals.md §4.1, §4.2).

use std::time::Instant;

use chrono::Utc;
use fa_domain::recipe::{RecipeDraft, RecipeOrigin};
use fa_domain::*;
use fa_recipe::RecipeError;
use fa_store::{
    AiSettingsRepo, IngredientRepo, LlmCallRepo, LlmEndpointRepo, NewIngredient, RecipeFields,
    RecipeImageRepo, RecipeLineRepo, RecipeLineRow, RecipeQuery, RecipeRatingRepo, RecipeRepo,
    RecipeRow, RecipeStepRepo, ReplacedLine,
};
use sha2::{Digest, Sha256};
use sqlx::PgConnection;
use url::Url;

use crate::error::{MealsError, Result};
use crate::image;
use crate::matching::IngredientMatcher;
use crate::service::{ActorRef, MealsService, enqueue, opt_u32, publish, to_i32};

const MAX_LINES: usize = 200;
const MAX_LINE_CHARS: usize = 500;
const MAX_STEPS: usize = 100;
const MAX_STEP_CHARS: usize = 5000;
const MAX_TAGS: usize = 30;

/// `GET /recipes` parameters.
#[derive(Debug, Clone, Default)]
pub struct RecipeListQuery {
    pub q: Option<String>,
    pub tag: Option<String>,
    pub favourite: Option<bool>,
    /// The opaque `next_cursor` of the previous page.
    pub before: Option<String>,
    /// Default 50, at most 200.
    pub limit: Option<u32>,
}

/// A stored image as the route serves it.
#[derive(Debug, Clone, PartialEq)]
pub struct RecipeImageData {
    pub mime: String,
    pub bytes: Vec<u8>,
    pub sha256: String,
}

fn hex(s: &str) -> String {
    s.bytes().map(|b| format!("{b:02x}")).collect()
}

fn unhex(s: &str) -> Option<String> {
    if !s.len().is_multiple_of(2) {
        return None;
    }
    let bytes: Option<Vec<u8>> = (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(s.get(i..i + 2)?, 16).ok())
        .collect();
    String::from_utf8(bytes?).ok()
}

fn encode_cursor(title_key: &str, id: RecipeId) -> String {
    hex(&format!("{id}\u{1f}{title_key}"))
}

fn decode_cursor(c: &str) -> Option<(String, RecipeId)> {
    let raw = unhex(c)?;
    let (id, key) = raw.split_once('\u{1f}')?;
    Some((key.to_string(), id.parse().ok()?))
}

fn check_opt_u32(field: &'static str, v: Option<u32>, max: u32) -> Result<Option<i32>> {
    match v {
        Some(n) if n > max => Err(MealsError::invalid(field, format!("at most {max}"))),
        other => Ok(other.map(to_i32)),
    }
}

fn clean_tags(tags: &[String]) -> Result<Vec<String>> {
    let mut out: Vec<String> = Vec::new();
    for t in tags {
        let t = t.trim();
        if t.is_empty() {
            continue;
        }
        if t.chars().count() > 40 {
            return Err(MealsError::invalid(
                "tags",
                "a tag has at most 40 characters",
            ));
        }
        if !out.iter().any(|o| o.eq_ignore_ascii_case(t)) {
            out.push(t.to_string());
        }
    }
    if out.len() > MAX_TAGS {
        return Err(MealsError::invalid(
            "tags",
            format!("at most {MAX_TAGS} tags"),
        ));
    }
    Ok(out)
}

fn http_url(field: &'static str, v: &str) -> Result<Url> {
    let u = Url::parse(v.trim()).map_err(|_| MealsError::invalid(field, "not a valid URL"))?;
    if !matches!(u.scheme(), "http" | "https") {
        return Err(MealsError::invalid(field, "only http and https URLs"));
    }
    Ok(u)
}

/// Validated, trimmed parts of a [`RecipeInput`].
struct Checked {
    fields: RecipeFields,
    lines: Vec<String>,
    steps: Vec<String>,
    image_url: Option<String>,
}

fn check_input(i: &RecipeInput) -> Result<Checked> {
    let title = i.title.trim();
    if title.is_empty() || title.chars().count() > 200 {
        return Err(MealsError::invalid("title", "1 to 200 characters"));
    }
    if matches!(i.servings, Some(0)) {
        return Err(MealsError::invalid("servings", "1 to 100"));
    }
    let servings = check_opt_u32("servings", i.servings, 100)?;
    let minutes = |f: &'static str, v| check_opt_u32(f, v, 100_000);
    let note = i
        .note
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string);
    if note.as_ref().is_some_and(|n| n.chars().count() > 5000) {
        return Err(MealsError::invalid("note", "at most 5000 characters"));
    }
    let source_url = match i
        .source_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(u) => Some(http_url("source_url", u)?.to_string()),
        None => None,
    };
    let image_url = match i
        .image_url
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        Some(u) => Some(http_url("image_url", u)?.to_string()),
        None => None,
    };
    let lines: Vec<String> = i
        .lines
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if lines.len() > MAX_LINES || lines.iter().any(|l| l.chars().count() > MAX_LINE_CHARS) {
        return Err(MealsError::invalid(
            "lines",
            format!("at most {MAX_LINES} lines of {MAX_LINE_CHARS} characters"),
        ));
    }
    let steps: Vec<String> = i
        .steps
        .iter()
        .map(|l| l.trim().to_string())
        .filter(|l| !l.is_empty())
        .collect();
    if steps.len() > MAX_STEPS || steps.iter().any(|l| l.chars().count() > MAX_STEP_CHARS) {
        return Err(MealsError::invalid(
            "steps",
            format!("at most {MAX_STEPS} steps of {MAX_STEP_CHARS} characters"),
        ));
    }
    Ok(Checked {
        fields: RecipeFields {
            title: title.to_string(),
            source_url,
            servings,
            yield_text: i
                .yield_text
                .as_deref()
                .map(str::trim)
                .filter(|s| !s.is_empty())
                .map(str::to_string),
            prep_minutes: minutes("prep_minutes", i.prep_minutes)?,
            cook_minutes: minutes("cook_minutes", i.cook_minutes)?,
            total_minutes: minutes("total_minutes", i.total_minutes)?,
            tags: clean_tags(&i.tags)?,
            note,
        },
        lines,
        steps,
        image_url,
    })
}

fn line_view(r: RecipeLineRow) -> RecipeLineView {
    RecipeLineView {
        id: r.id,
        ord: u32::try_from(r.ord).unwrap_or(0),
        raw_text: r.raw_text,
        qty: r.qty,
        unit: r.unit,
        item: r.item,
        note: r.note,
        ingredient_id: r.ingredient_id,
        ingredient_name: r.ingredient_name,
        match_state: r.match_state,
        proposal: r
            .proposal
            .and_then(|p| serde_json::from_value::<IngredientProposal>(p).ok()),
    }
}

async fn ratings_view(
    conn: &mut PgConnection,
    recipe: RecipeId,
    me: Option<MemberId>,
) -> Result<RatingsView> {
    let all = RecipeRatingRepo::for_recipe(conn, recipe).await?;
    let avg = (!all.is_empty())
        .then(|| all.iter().map(|(_, s)| f32::from(*s)).sum::<f32>() / all.len() as f32);
    let mine = me.and_then(|m| {
        all.iter()
            .find(|(i, _)| *i == m)
            .and_then(|(_, s)| u8::try_from(*s).ok())
    });
    Ok(RatingsView {
        avg,
        mine,
        by_member: all
            .into_iter()
            .filter_map(|(m, s)| {
                Some(MemberRating {
                    member_id: m,
                    stars: u8::try_from(s).ok()?,
                })
            })
            .collect(),
    })
}

async fn view(conn: &mut PgConnection, r: RecipeRow, me: Option<MemberId>) -> Result<RecipeView> {
    let lines = RecipeLineRepo::for_recipe(conn, r.id).await?;
    let steps = RecipeStepRepo::for_recipe(conn, r.id).await?;
    let ratings = ratings_view(conn, r.id, me).await?;
    let f = r.fields;
    Ok(RecipeView {
        id: r.id,
        title: f.title,
        source_url: f.source_url,
        servings: opt_u32(f.servings),
        yield_text: f.yield_text,
        prep_minutes: opt_u32(f.prep_minutes),
        cook_minutes: opt_u32(f.cook_minutes),
        total_minutes: opt_u32(f.total_minutes),
        import_method: r.import_method,
        tags: f.tags,
        note: f.note,
        favourite: r.favourite,
        has_image: r.has_image,
        lines: lines.into_iter().map(line_view).collect(),
        steps,
        ratings,
        version: r.version,
    })
}

impl MealsService {
    // ---- library ----

    pub async fn list_recipes(
        &self,
        h: HouseholdId,
        me: Option<MemberId>,
        q: RecipeListQuery,
    ) -> Result<RecipePage> {
        let limit = i64::from(q.limit.unwrap_or(50).clamp(1, 200));
        let after = match q.before.as_deref().filter(|s| !s.is_empty()) {
            Some(c) => Some(
                decode_cursor(c)
                    .ok_or_else(|| MealsError::invalid("before", "not a valid cursor"))?,
            ),
            None => None,
        };
        let mut tx = self.store.tx_for(h).await?;
        let ctx = self.ctx(&mut tx, h).await?;
        let mut rows = RecipeRepo::list(
            &mut tx,
            &RecipeQuery {
                q: q.q.map(|s| s.trim().to_string()),
                tag: q.tag.filter(|t| !t.is_empty()),
                favourite: q.favourite,
                me,
                today: Some(self.today(&ctx)),
                after,
                limit: limit + 1,
            },
        )
        .await?;
        tx.rollback().await?;
        let more = rows.len() as i64 > limit;
        rows.truncate(limit as usize);
        let next_cursor = if more {
            rows.last().map(|r| encode_cursor(&r.title_key, r.id))
        } else {
            None
        };
        Ok(RecipePage {
            items: rows
                .into_iter()
                .map(|r| RecipeSummary {
                    id: r.id,
                    title: r.title,
                    has_image: r.has_image,
                    total_minutes: opt_u32(r.total_minutes),
                    tags: r.tags,
                    favourite: r.favourite,
                    rating_avg: r.rating_avg,
                    my_rating: r.my_rating.and_then(|s| u8::try_from(s).ok()),
                    last_planned: r.last_planned,
                })
                .collect(),
            next_cursor,
        })
    }

    /// Every tag in the library, for the filter chips.
    pub async fn recipe_tags(&self, h: HouseholdId) -> Result<Vec<String>> {
        let mut tx = self.store.tx_for(h).await?;
        let tags = RecipeRepo::all_tags(&mut tx).await?;
        tx.rollback().await?;
        Ok(tags)
    }

    pub async fn get_recipe(
        &self,
        h: HouseholdId,
        me: Option<MemberId>,
        id: RecipeId,
    ) -> Result<RecipeView> {
        let mut tx = self.store.tx_for(h).await?;
        let row = RecipeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let v = view(&mut tx, row, me).await?;
        tx.rollback().await?;
        Ok(v)
    }

    // ---- write ----

    /// Parses, cleans and matches every line, replaces them, and queues `ingredient.llm` for the
    /// lines the rules could not match. Returns the stored lines.
    async fn store_lines(
        &self,
        conn: &mut PgConnection,
        h: HouseholdId,
        recipe: RecipeId,
        raw: &[String],
        llm: bool,
    ) -> Result<Vec<ReplacedLine>> {
        let (settings, n) = self.settings_and_normaliser(conn, h).await?;
        let matcher = IngredientMatcher::new(settings.trigram_threshold);
        let mut new = Vec::with_capacity(raw.len());
        for l in raw {
            new.push(matcher.line(conn, &n, l).await?);
        }
        let stored = RecipeLineRepo::replace_for_recipe(conn, recipe, &new).await?;
        if settings.ingredient_llm && llm {
            for l in &stored {
                if !l.kept && l.row.match_state == MatchState::Unmatched && l.row.item.is_some() {
                    enqueue(
                        conn,
                        h,
                        "ingredient.llm",
                        serde_json::json!({"household": h, "line_id": l.row.id}),
                        format!("ingredient.llm:{}", l.row.id),
                    )
                    .await?;
                }
            }
        }
        Ok(stored)
    }

    async fn queue_image(
        &self,
        conn: &mut PgConnection,
        h: HouseholdId,
        recipe: RecipeId,
        url: &str,
    ) -> Result<()> {
        let sha = format!("{:x}", Sha256::digest(url.as_bytes()));
        enqueue(
            conn,
            h,
            "recipe.image",
            serde_json::json!({"household": h, "recipe_id": recipe, "url": url}),
            format!("recipe.image:{recipe}:{}", &sha[..16]),
        )
        .await?;
        Ok(())
    }

    pub async fn create_recipe(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        input: RecipeInput,
    ) -> Result<RecipeView> {
        let c = check_input(&input)?;
        let llm = self.llm_ready(h).await;
        let mut tx = self.store.tx_for(h).await?;
        let id = RecipeId::new();
        let row = RecipeRepo::insert(&mut tx, id, h, &c.fields, input.import_method, actor.member)
            .await?;
        self.store_lines(&mut tx, h, id, &c.lines, llm).await?;
        RecipeStepRepo::replace_for_recipe(&mut tx, id, &c.steps).await?;
        if let Some(u) = &c.image_url {
            self.queue_image(&mut tx, h, id, u).await?;
        }
        publish(&mut tx, h, "recipes.updated").await?;
        let row = RecipeRepo::get(&mut tx, row.id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let v = view(&mut tx, row, actor.member).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Lines are replaced whole; a line whose `raw_text` is unchanged keeps its match.
    pub async fn update_recipe(
        &self,
        h: HouseholdId,
        actor: ActorRef,
        id: RecipeId,
        input: RecipeInput,
        if_match: i32,
    ) -> Result<RecipeView> {
        let c = check_input(&input)?;
        let llm = self.llm_ready(h).await;
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::update(&mut tx, id, &c.fields, if_match).await?;
        self.store_lines(&mut tx, h, id, &c.lines, llm).await?;
        RecipeStepRepo::replace_for_recipe(&mut tx, id, &c.steps).await?;
        if let Some(u) = &c.image_url {
            self.queue_image(&mut tx, h, id, u).await?;
        }
        publish(&mut tx, h, "recipes.updated").await?;
        let row = RecipeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        let v = view(&mut tx, row, actor.member).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Plan entries that use the recipe keep its title as free text, in the same transaction.
    pub async fn delete_recipe(&self, h: HouseholdId, id: RecipeId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::delete(&mut tx, id).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        publish(&mut tx, h, "mealplan.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn set_favourite(&self, h: HouseholdId, id: RecipeId, on: bool) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::set_favourite(&mut tx, id, on).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    /// The member's own rating; `None` clears it.
    pub async fn set_rating(
        &self,
        h: HouseholdId,
        member: MemberId,
        id: RecipeId,
        stars: Option<u8>,
    ) -> Result<RatingsView> {
        if stars.is_some_and(|s| !(1..=5).contains(&s)) {
            return Err(MealsError::invalid("stars", "1 to 5"));
        }
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        match stars {
            Some(s) => RecipeRatingRepo::set(&mut tx, id, member, i16::from(s)).await?,
            None => RecipeRatingRepo::clear(&mut tx, id, member).await?,
        }
        publish(&mut tx, h, "recipes.updated").await?;
        let v = ratings_view(&mut tx, id, Some(member)).await?;
        tx.commit().await?;
        Ok(v)
    }

    /// Accepts a proposal or picks another match. The line's cleaned item becomes an alias, so
    /// the next import matches by rules.
    pub async fn match_line(
        &self,
        h: HouseholdId,
        recipe: RecipeId,
        line: RecipeLineId,
        req: LineMatchReq,
    ) -> Result<RecipeLineView> {
        let mut tx = self.store.tx_for(h).await?;
        let row = RecipeLineRepo::get(&mut tx, line)
            .await?
            .filter(|l| l.recipe_id == recipe)
            .ok_or(MealsError::NotFound)?;
        let ingredient = match (req.ingredient_id, req.new_name.as_deref()) {
            (Some(i), _) => IngredientRepo::get(&mut tx, i)
                .await?
                .ok_or_else(|| MealsError::invalid("ingredient_id", "unknown ingredient"))?,
            (None, Some(name)) => {
                let (_, n) = self.settings_and_normaliser(&mut tx, h).await?;
                let name = n.clean_item(name).0;
                if name.is_empty() || name.chars().count() > 100 {
                    return Err(MealsError::invalid("new_name", "1 to 100 characters"));
                }
                match IngredientRepo::match_exact(&mut tx, &name).await? {
                    Some(i) => i,
                    None => {
                        let proposed = row
                            .proposal
                            .as_ref()
                            .and_then(|p| p.get("new_name"))
                            .and_then(|v| v.as_str())
                            .is_some_and(|p| n.clean_item(p).0 == name);
                        if let Some(a) = req.aisle_id
                            && fa_store::AisleRepo::list(&mut tx)
                                .await?
                                .iter()
                                .all(|x| x.id != a)
                        {
                            return Err(MealsError::invalid("aisle_id", "unknown aisle"));
                        }
                        IngredientRepo::insert(
                            &mut tx,
                            h,
                            &NewIngredient {
                                canonical_name: name,
                                aliases: Vec::new(),
                                is_staple: false,
                                aisle_id: req.aisle_id,
                                default_store_id: None,
                                default_unit: None,
                                origin: if proposed {
                                    IngredientOrigin::Llm
                                } else {
                                    IngredientOrigin::User
                                },
                            },
                        )
                        .await?
                    }
                }
            }
            (None, None) => {
                return Err(MealsError::invalid(
                    "ingredient_id",
                    "send ingredient_id or new_name",
                ));
            }
        };
        if let Some(item) = row.item.as_deref() {
            IngredientRepo::add_alias(&mut tx, ingredient.id, item).await?;
        }
        RecipeLineRepo::set_match(
            &mut tx,
            line,
            Some(ingredient.id),
            MatchState::Matched,
            None,
        )
        .await?;
        publish(&mut tx, h, "recipes.updated").await?;
        let out = RecipeLineRepo::get(&mut tx, line)
            .await?
            .ok_or(MealsError::NotFound)?;
        tx.commit().await?;
        Ok(line_view(out))
    }

    // ---- images ----

    pub async fn get_image(&self, h: HouseholdId, id: RecipeId) -> Result<Option<RecipeImageData>> {
        let mut tx = self.store.tx_for(h).await?;
        let img = RecipeImageRepo::get(&mut tx, id).await?;
        tx.rollback().await?;
        Ok(img.map(|i| RecipeImageData {
            mime: i.mime,
            bytes: i.bytes,
            sha256: i.sha256,
        }))
    }

    /// The stored sha256 only, for `If-None-Match`.
    pub async fn image_sha(&self, h: HouseholdId, id: RecipeId) -> Result<Option<String>> {
        let mut tx = self.store.tx_for(h).await?;
        let sha = RecipeImageRepo::sha(&mut tx, id).await?;
        tx.rollback().await?;
        Ok(sha)
    }

    /// An uploaded image (png, jpeg or webp, at most 5 MB), resized and re-encoded.
    pub async fn put_image(&self, h: HouseholdId, id: RecipeId, bytes: Vec<u8>) -> Result<()> {
        let img = tokio::task::spawn_blocking(move || image::process(&bytes))
            .await
            .map_err(|e| MealsError::Image(e.to_string()))??;
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        RecipeImageRepo::put(&mut tx, id, img.mime, &img.bytes, &img.sha256, "upload").await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    pub async fn delete_image(&self, h: HouseholdId, id: RecipeId) -> Result<()> {
        let mut tx = self.store.tx_for(h).await?;
        RecipeRepo::get(&mut tx, id)
            .await?
            .ok_or(MealsError::NotFound)?;
        RecipeImageRepo::delete(&mut tx, id).await?;
        publish(&mut tx, h, "recipes.updated").await?;
        tx.commit().await?;
        Ok(())
    }

    // ---- import ----

    /// `{url}` or `{text}` to an editable draft. Nothing is stored.
    pub async fn import(&self, h: HouseholdId, req: RecipeImportReq) -> Result<RecipeDraft> {
        let url = req.url.as_deref().map(str::trim).filter(|s| !s.is_empty());
        let text = req.text.as_deref().map(str::trim).filter(|s| !s.is_empty());
        match (url, text) {
            (Some(u), None) => self.import_url(h, http_url("url", u)?).await,
            (None, Some(t)) => self.llm_extract(h, t, None).await,
            _ => Err(MealsError::invalid(
                "url",
                "send exactly one of url and text",
            )),
        }
    }

    async fn import_url(&self, h: HouseholdId, url: Url) -> Result<RecipeDraft> {
        tracing::info!(host = url.host_str().unwrap_or(""), "recipe fetch");
        let http = fa_connectors::Http::with_policy(true, self.allow_private_hosts)
            .map_err(|e| MealsError::FetchFailed(e.to_string()))?;
        let html = fa_recipe::fetch_page(&http, &url)
            .await
            .map_err(|e| match e {
                RecipeError::NotHtml => MealsError::FetchFailed("the page is not html".into()),
                other => MealsError::FetchFailed(other.to_string()),
            })?;
        if let Some(d) = fa_recipe::from_html(&html, Some(&url)) {
            return Ok(d);
        }
        if let Some(d) = fa_recipe::from_microdata(&html, Some(&url)) {
            return Ok(d);
        }
        let text = fa_recipe::visible_text(&html);
        self.llm_extract(h, &text, Some(&url)).await
    }

    /// The LLM path of an import: truncates to `max_input_chars`, records the call.
    async fn llm_extract(
        &self,
        h: HouseholdId,
        text: &str,
        source: Option<&Url>,
    ) -> Result<RecipeDraft> {
        let Some(provider) = &self.llm else {
            return Err(MealsError::LlmUnavailable);
        };
        let client = provider
            .client_for(h)
            .await
            .map_err(|e| MealsError::LlmFailed(e.to_string()))?
            .ok_or(MealsError::LlmUnavailable)?;
        let mut tx = self.store.tx_for(h).await?;
        let max = AiSettingsRepo::get_or_default(&mut tx, h)
            .await?
            .max_input_chars as usize;
        let model = LlmEndpointRepo::get(&mut tx, h)
            .await?
            .map(|e| e.model)
            .unwrap_or_default();
        tx.rollback().await?;
        let text: String = text.chars().take(max).collect();
        let started_at = Utc::now();
        let t0 = Instant::now();
        let res = fa_recipe::from_llm(&client, &text, source).await;
        let latency = t0.elapsed();
        // "no recipe" is a model answer, not a transport failure.
        let transport_err = res
            .as_ref()
            .err()
            .filter(|e| !matches!(e, fa_llm::LlmError::Schema(m) if m == "no recipe"));
        let rec = NewLlmCall {
            household_id: h,
            purpose: "recipe.extract".to_owned(),
            model,
            started_at,
            latency_ms: i32::try_from(latency.as_millis()).unwrap_or(i32::MAX),
            ok: transport_err.is_none(),
            error_kind: transport_err.map(|e| e.kind().to_owned()),
            prompt_tokens: None,
            completion_tokens: None,
        };
        let recorded = async {
            let mut tx = self.store.tx_for(h).await?;
            LlmCallRepo::insert(&mut tx, &rec).await?;
            tx.commit().await
        }
        .await;
        if let Err(e) = recorded {
            tracing::warn!(error = %e, "llm_call not recorded");
        }
        match res {
            Ok(mut d) => {
                d.origin = RecipeOrigin::Llm;
                Ok(d)
            }
            Err(fa_llm::LlmError::Schema(m)) if m == "no recipe" => Err(MealsError::NoRecipe),
            Err(e) => Err(MealsError::LlmFailed(e.to_string())),
        }
    }
}
