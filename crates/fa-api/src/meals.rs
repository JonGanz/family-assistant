//! M5 routes: recipes, ingredients, meal settings, the meal plan, templates and the grocery list
//! (plan/M5.md, API table). Handlers stay thin: authorise, then call the `MealsService`.

use axum::Json;
use axum::body::Bytes;
use axum::extract::State;
use axum::http::header::{CACHE_CONTROL, CONTENT_TYPE, ETAG, IF_NONE_MATCH};
use axum::http::{HeaderMap, HeaderName, HeaderValue, StatusCode};
use axum::response::{IntoResponse, Response};
use chrono::NaiveDate;
use fa_domain::recipe::RecipeDraft;
use fa_domain::*;
use fa_meals::{ActorRef, RecipeListQuery};
use fa_store::ChangeBus;
use serde::{Deserialize, Serialize};
use serde_json::json;
use utoipa::{IntoParams, ToSchema};

use crate::error::Problem;
use crate::extract::{ApiJson, ApiPath, ApiQuery};
use crate::routes::{ApiResult, etag, household, parse_version};
use crate::tasks::{actor_member, device_of};
use crate::{ApiError, AppState, Auth, require};

/// Largest accepted recipe image upload.
pub const IMAGE_MAX_BYTES: usize = 5 * 1024 * 1024;
const NOSNIFF: HeaderName = HeaderName::from_static("x-content-type-options");

fn actor_ref(actor: &Actor) -> ActorRef {
    ActorRef {
        member: actor_member(actor),
        device: device_of(actor),
    }
}

fn read(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ReadMeals, &Resource::Meals)
}

fn manage(actor: &Actor) -> ApiResult<()> {
    require(actor, Action::ManageMeals, &Resource::Meals)
}

fn with_etag<T: Serialize>(version: i32, v: T) -> Response {
    ([(ETAG, etag(version))], Json(v)).into_response()
}

fn entry_version(r: &PlanChangeResult, id: Option<MealPlanEntryId>) -> Option<i32> {
    let id = id?;
    r.entries.iter().find(|e| e.id == id).map(|e| e.version)
}

fn plan_response(status: StatusCode, id: Option<MealPlanEntryId>, r: PlanChangeResult) -> Response {
    let tag = entry_version(&r, id);
    let mut res = (status, Json(r)).into_response();
    if let Some(v) = tag {
        res.headers_mut().insert(ETAG, etag(v));
    }
    res
}

// ---- recipes ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct RecipeQuery {
    /// Title or ingredient search.
    q: Option<String>,
    tag: Option<String>,
    favourite: Option<bool>,
    /// The opaque `next_cursor` of the previous page.
    before: Option<String>,
    /// Page size, default 50, at most 200.
    limit: Option<u32>,
}

#[utoipa::path(get, path = "/api/v1/recipes", tag = "meals", params(RecipeQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RecipePage), (status = 403, body = Problem)))]
pub async fn list_recipes(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<RecipeQuery>,
) -> ApiResult<Json<RecipePage>> {
    read(&auth.actor)?;
    let page = s
        .services
        .meals
        .list_recipes(
            household(&auth.actor)?,
            actor_member(&auth.actor),
            RecipeListQuery {
                q: q.q,
                tag: q.tag,
                favourite: q.favourite,
                before: q.before,
                limit: q.limit,
            },
        )
        .await?;
    Ok(Json(page))
}

#[utoipa::path(get, path = "/api/v1/recipes/tags", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<String>), (status = 403, body = Problem)))]
pub async fn list_recipe_tags(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<String>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .recipe_tags(household(&auth.actor)?)
            .await?,
    ))
}

/// `image_url` queues the `recipe.image` job.
#[utoipa::path(post, path = "/api/v1/recipes", tag = "meals", request_body = RecipeInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = RecipeView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn create_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<RecipeInput>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .create_recipe(household(&auth.actor)?, actor_ref(&auth.actor), input)
        .await?;
    let mut res = (StatusCode::CREATED, Json(&v)).into_response();
    res.headers_mut().insert(ETAG, etag(v.version));
    Ok(res)
}

#[utoipa::path(get, path = "/api/v1/recipes/{id}", tag = "meals",
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RecipeView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn get_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
) -> ApiResult<Response> {
    read(&auth.actor)?;
    let v = s
        .services
        .meals
        .get_recipe(household(&auth.actor)?, actor_member(&auth.actor), id)
        .await?;
    Ok(with_etag(v.version, v))
}

/// The lines are replaced whole: unchanged `raw_text` lines keep their match.
#[utoipa::path(put, path = "/api/v1/recipes/{id}", tag = "meals", request_body = RecipeInput,
    params(("id" = String, Path, description = "recipe id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RecipeView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn put_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<RecipeInput>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .meals
        .update_recipe(
            household(&auth.actor)?,
            actor_ref(&auth.actor),
            id,
            input,
            expected,
        )
        .await?;
    Ok(with_etag(v.version, v))
}

/// Plan entries keep the recipe title as free text.
#[utoipa::path(delete, path = "/api/v1/recipes/{id}", tag = "meals",
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    s.services
        .meals
        .delete_recipe(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, ToSchema)]
pub struct FavouriteReq {
    pub favourite: bool,
}

#[utoipa::path(put, path = "/api/v1/recipes/{id}/favourite", tag = "meals", request_body = FavouriteReq,
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn put_favourite(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
    ApiJson(b): ApiJson<FavouriteReq>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    s.services
        .meals
        .set_favourite(household(&auth.actor)?, id, b.favourite)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[derive(Deserialize, ToSchema)]
pub struct RatingReq {
    /// 1 to 5, or null to remove the rating.
    #[serde(default)]
    pub stars: Option<u8>,
}

/// A member rates for themself; devices cannot rate.
#[utoipa::path(put, path = "/api/v1/recipes/{id}/rating", tag = "meals", request_body = RatingReq,
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RatingsView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn put_rating(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
    ApiJson(b): ApiJson<RatingReq>,
) -> ApiResult<Json<RatingsView>> {
    require(&auth.actor, Action::RateRecipe, &Resource::Meals)?;
    let me = actor_member(&auth.actor).ok_or(ApiError::Forbidden("forbidden".into()))?;
    Ok(Json(
        s.services
            .meals
            .set_rating(household(&auth.actor)?, me, id, b.stars)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/recipes/{id}/image", tag = "meals",
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, content_type = "image/*", description = "the image; ETag is its sha256"),
              (status = 304), (status = 404, body = Problem)))]
pub async fn get_recipe_image(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
    headers: HeaderMap,
) -> ApiResult<Response> {
    read(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let cache = (
        CACHE_CONTROL,
        HeaderValue::from_static("private, max-age=86400"),
    );
    // The hash alone answers a conditional request without loading the bytes.
    let sha = s
        .services
        .meals
        .image_sha(hid, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let tag = format!("\"{sha}\"");
    let etag_value = HeaderValue::from_str(&tag).map_err(|e| ApiError::Internal(e.to_string()))?;
    let fresh = headers
        .get(IF_NONE_MATCH)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| {
            v.split(',')
                .any(|t| t.trim().trim_start_matches("W/") == tag)
        });
    if fresh {
        return Ok((StatusCode::NOT_MODIFIED, [cache, (ETAG, etag_value)]).into_response());
    }
    let img = s
        .services
        .meals
        .get_image(hid, id)
        .await?
        .ok_or(ApiError::NotFound)?;
    let ct = HeaderValue::from_str(&img.mime).map_err(|e| ApiError::Internal(e.to_string()))?;
    Ok((
        [
            (CONTENT_TYPE, ct),
            (ETAG, etag_value),
            cache,
            (NOSNIFF, HeaderValue::from_static("nosniff")),
        ],
        img.bytes,
    )
        .into_response())
}

#[utoipa::path(post, path = "/api/v1/recipes/{id}/image", tag = "meals",
    params(("id" = String, Path, description = "recipe id")),
    request_body(content = Vec<u8>, content_type = "image/png",
        description = "raw image bytes (png, jpeg or webp), at most 5 MB; stored resized"),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 413, body = Problem), (status = 415, body = Problem),
              (status = 422, body = Problem, description = "code `bad_image`")))]
pub async fn post_recipe_image(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
    headers: HeaderMap,
    body: Result<Bytes, axum::extract::rejection::BytesRejection>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    // The declared type is only a gate; the real type comes from the magic bytes.
    let declared = headers
        .get(CONTENT_TYPE)
        .and_then(|v| v.to_str().ok())
        .map(|v| {
            v.split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        });
    if !matches!(
        declared.as_deref(),
        Some("image/png" | "image/jpeg" | "image/webp")
    ) {
        return Err(ApiError::UnsupportedMedia(
            "send image/png, image/jpeg or image/webp".into(),
        ));
    }
    let bytes = body.map_err(|r| {
        if r.status() == StatusCode::PAYLOAD_TOO_LARGE {
            ApiError::PayloadTooLarge(format!("the image must be at most {IMAGE_MAX_BYTES} bytes"))
        } else {
            ApiError::BadRequest("could not read the request body".into())
        }
    })?;
    if bytes.len() > IMAGE_MAX_BYTES {
        return Err(ApiError::PayloadTooLarge(format!(
            "the image must be at most {IMAGE_MAX_BYTES} bytes"
        )));
    }
    if fa_meals::image::sniff_image(&bytes).is_none() {
        return Err(ApiError::UnsupportedMedia(
            "the file is not a png, jpeg or webp image".into(),
        ));
    }
    s.services
        .meals
        .put_image(household(&auth.actor)?, id, bytes.to_vec())
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(delete, path = "/api/v1/recipes/{id}/image", tag = "meals",
    params(("id" = String, Path, description = "recipe id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_recipe_image(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<RecipeId>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    s.services
        .meals
        .delete_image(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// `{url}` or `{text}` to an editable draft; nothing is stored.
#[utoipa::path(post, path = "/api/v1/recipes/import", tag = "meals", request_body = RecipeImportReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RecipeDraft), (status = 403, body = Problem),
              (status = 422, body = Problem, description = "codes `no_recipe`, `fetch_failed`"),
              (status = 502, body = Problem, description = "code `llm_failed`"),
              (status = 503, body = Problem, description = "code `llm_unavailable`")))]
pub async fn import_recipe(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<RecipeImportReq>,
) -> ApiResult<Json<RecipeDraft>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .import(household(&auth.actor)?, req)
            .await?,
    ))
}

/// Adds the line's `item` as an alias of the chosen ingredient.
#[utoipa::path(post, path = "/api/v1/recipes/{id}/lines/{line_id}/match", tag = "meals",
    request_body = LineMatchReq,
    params(("id" = String, Path, description = "recipe id"),
           ("line_id" = String, Path, description = "ingredient line id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = RecipeLineView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn match_recipe_line(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath((id, line)): ApiPath<(RecipeId, RecipeLineId)>,
    ApiJson(req): ApiJson<LineMatchReq>,
) -> ApiResult<Json<RecipeLineView>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .match_line(household(&auth.actor)?, id, line, req)
            .await?,
    ))
}

// ---- ingredients ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct IngredientQuery {
    /// Filters by trigram and alias.
    q: Option<String>,
}

#[utoipa::path(get, path = "/api/v1/ingredients", tag = "meals", params(IngredientQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<IngredientView>), (status = 403, body = Problem)))]
pub async fn list_ingredients(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<IngredientQuery>,
) -> ApiResult<Json<Vec<IngredientView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .list_ingredients(household(&auth.actor)?, q.q.as_deref())
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/ingredients", tag = "meals", request_body = IngredientInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = IngredientView), (status = 403, body = Problem),
              (status = 409, body = Problem), (status = 422, body = Problem)))]
pub async fn create_ingredient(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<IngredientInput>,
) -> ApiResult<(StatusCode, Json<IngredientView>)> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .create_ingredient(household(&auth.actor)?, input)
        .await?;
    Ok((StatusCode::CREATED, Json(v)))
}

#[utoipa::path(put, path = "/api/v1/ingredients/{id}", tag = "meals", request_body = IngredientInput,
    params(("id" = String, Path, description = "ingredient id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = IngredientView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_ingredient(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<IngredientId>,
    ApiJson(input): ApiJson<IngredientInput>,
) -> ApiResult<Json<IngredientView>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .update_ingredient(household(&auth.actor)?, id, input)
            .await?,
    ))
}

/// Lines and items lose the link.
#[utoipa::path(delete, path = "/api/v1/ingredients/{id}", tag = "meals",
    params(("id" = String, Path, description = "ingredient id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_ingredient(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<IngredientId>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    s.services
        .meals
        .delete_ingredient(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/ingredients/{id}/merge", tag = "meals",
    request_body = IngredientMergeReq,
    params(("id" = String, Path, description = "ingredient id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = IngredientView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn merge_ingredient(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<IngredientId>,
    ApiJson(b): ApiJson<IngredientMergeReq>,
) -> ApiResult<Json<IngredientView>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .merge_ingredient(household(&auth.actor)?, id, b.into)
            .await?,
    ))
}

// ---- settings, slots, aisles, stores ----

#[utoipa::path(get, path = "/api/v1/meals/settings", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MealsSettingsView), (status = 403, body = Problem)))]
pub async fn get_meals_settings(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<MealsSettingsView>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .get_settings(household(&auth.actor)?)
            .await?,
    ))
}

/// Absent fields keep their value. The horizon (1 to 52 weeks) is stored on the household.
#[utoipa::path(put, path = "/api/v1/meals/settings", tag = "meals", request_body = MealsSettingsPatch,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MealsSettingsView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_meals_settings(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(p): ApiJson<MealsSettingsPatch>,
) -> ApiResult<Json<MealsSettingsView>> {
    manage(&auth.actor)?;
    let hid = household(&auth.actor)?;
    let horizon = p.meal_plan_horizon_weeks.is_some();
    let v = s.services.meals.update_settings(hid, p).await?;
    if horizon {
        let mut tx = s.store.tx_for(hid).await?;
        ChangeBus::publish(&mut tx, hid, "household.updated", &json!({})).await?;
        tx.commit().await?;
    }
    Ok(Json(v))
}

#[utoipa::path(get, path = "/api/v1/meals/slots", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<MealSlotView>), (status = 403, body = Problem)))]
pub async fn get_meal_slots(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<MealSlotView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services.meals.list_slots(household(&auth.actor)?).await?,
    ))
}

/// Replaces the ordered list. A slot in use is removed with 409 `slot_in_use`.
#[utoipa::path(put, path = "/api/v1/meals/slots", tag = "meals", request_body = Vec<MealSlotView>,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<MealSlotView>), (status = 403, body = Problem),
              (status = 409, body = Problem, description = "code `slot_in_use`"),
              (status = 422, body = Problem)))]
pub async fn put_meal_slots(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(slots): ApiJson<Vec<MealSlotView>>,
) -> ApiResult<Json<Vec<MealSlotView>>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .replace_slots(household(&auth.actor)?, slots)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/meals/aisles", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<AisleView>), (status = 403, body = Problem)))]
pub async fn get_meal_aisles(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<AisleView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .list_aisles(household(&auth.actor)?)
            .await?,
    ))
}

/// Replaces the list; "Other" is implicit.
#[utoipa::path(put, path = "/api/v1/meals/aisles", tag = "meals", request_body = Vec<AisleView>,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<AisleView>), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_meal_aisles(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(aisles): ApiJson<Vec<AisleView>>,
) -> ApiResult<Json<Vec<AisleView>>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .replace_aisles(household(&auth.actor)?, aisles)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/meals/stores", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<StoreView>), (status = 403, body = Problem)))]
pub async fn get_meal_stores(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<StoreView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .list_stores(household(&auth.actor)?)
            .await?,
    ))
}

/// Replaces the list; removing a store clears it from its items and ingredients.
#[utoipa::path(put, path = "/api/v1/meals/stores", tag = "meals", request_body = Vec<StoreView>,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<StoreView>), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn put_meal_stores(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(stores): ApiJson<Vec<StoreView>>,
) -> ApiResult<Json<Vec<StoreView>>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .replace_stores(household(&auth.actor)?, stores)
            .await?,
    ))
}

#[utoipa::path(get, path = "/api/v1/meals/stores/{id}/aisles", tag = "meals",
    params(("id" = String, Path, description = "store id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<StoreAisleView>), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn get_store_aisles(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryStoreId>,
) -> ApiResult<Json<Vec<StoreAisleView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .store_aisles(household(&auth.actor)?, id)
            .await?,
    ))
}

/// Every aisle of the household, in this store's order, with `hidden`.
#[utoipa::path(put, path = "/api/v1/meals/stores/{id}/aisles", tag = "meals",
    request_body = Vec<StoreAisleView>,
    params(("id" = String, Path, description = "store id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<StoreAisleView>), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn put_store_aisles(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryStoreId>,
    ApiJson(list): ApiJson<Vec<StoreAisleView>>,
) -> ApiResult<Json<Vec<StoreAisleView>>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .set_store_aisles(household(&auth.actor)?, id, list)
            .await?,
    ))
}

// ---- meal plan ----

#[derive(Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct PlanQuery {
    /// `YYYY-MM-DD`.
    #[param(value_type = String)]
    from: NaiveDate,
    /// `YYYY-MM-DD`, at most 62 days from `from`.
    #[param(value_type = String)]
    to: NaiveDate,
}

#[utoipa::path(get, path = "/api/v1/meal-plan", tag = "meals", params(PlanQuery),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<MealPlanEntryView>), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn get_meal_plan(
    State(s): State<AppState>,
    auth: Auth,
    ApiQuery(q): ApiQuery<PlanQuery>,
) -> ApiResult<Json<Vec<MealPlanEntryView>>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .plan_range(household(&auth.actor)?, q.from, q.to)
            .await?,
    ))
}

/// Appended to the slot. Several entries may share a slot.
#[utoipa::path(post, path = "/api/v1/meal-plan", tag = "meals", request_body = MealPlanEntryInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = PlanChangeResult), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn add_plan_entry(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<MealPlanEntryInput>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let r = s
        .services
        .meals
        .plan_add(household(&auth.actor)?, actor_ref(&auth.actor), input)
        .await?;
    Ok(plan_response(StatusCode::CREATED, None, r))
}

/// Moving to another date or slot appends there.
#[utoipa::path(put, path = "/api/v1/meal-plan/{id}", tag = "meals", request_body = MealPlanEntryPatch,
    params(("id" = String, Path, description = "plan entry id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = PlanChangeResult), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn put_plan_entry(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealPlanEntryId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<MealPlanEntryPatch>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let r = s
        .services
        .meals
        .plan_update(
            household(&auth.actor)?,
            actor_ref(&auth.actor),
            id,
            patch,
            expected,
        )
        .await?;
    Ok(plan_response(StatusCode::OK, Some(id), r))
}

#[utoipa::path(delete, path = "/api/v1/meal-plan/{id}", tag = "meals",
    params(("id" = String, Path, description = "plan entry id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = PlanChangeResult), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn delete_plan_entry(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealPlanEntryId>,
) -> ApiResult<Json<PlanChangeResult>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .plan_delete(household(&auth.actor)?, actor_ref(&auth.actor), id)
            .await?,
    ))
}

/// `{a, b}` swaps two entries; `{a, date, slot_id}` moves `a` into an empty slot.
#[utoipa::path(post, path = "/api/v1/meal-plan/swap", tag = "meals", request_body = SwapReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = PlanChangeResult), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn swap_plan_entries(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<SwapReq>,
) -> ApiResult<Json<PlanChangeResult>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .plan_swap(household(&auth.actor)?, actor_ref(&auth.actor), req)
            .await?,
    ))
}

// ---- templates ----

#[utoipa::path(get, path = "/api/v1/meal-templates", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = Vec<MealTemplateSummary>), (status = 403, body = Problem)))]
pub async fn list_templates(
    State(s): State<AppState>,
    auth: Auth,
) -> ApiResult<Json<Vec<MealTemplateSummary>>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .template_list(household(&auth.actor)?)
            .await?,
    ))
}

#[utoipa::path(post, path = "/api/v1/meal-templates", tag = "meals", request_body = MealTemplateInput,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = MealTemplateView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn create_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(input): ApiJson<MealTemplateInput>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .template_create(household(&auth.actor)?, input)
        .await?;
    Ok(created_template(v))
}

fn created_template(v: MealTemplateView) -> Response {
    let mut res = (StatusCode::CREATED, Json(&v)).into_response();
    res.headers_mut().insert(ETAG, etag(v.version));
    res
}

#[utoipa::path(get, path = "/api/v1/meal-templates/{id}", tag = "meals",
    params(("id" = String, Path, description = "template id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MealTemplateView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn get_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .template_get(household(&auth.actor)?, id)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(put, path = "/api/v1/meal-templates/{id}", tag = "meals", request_body = MealTemplateInput,
    params(("id" = String, Path, description = "template id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = MealTemplateView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn put_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
    headers: HeaderMap,
    ApiJson(input): ApiJson<MealTemplateInput>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .meals
        .template_update(household(&auth.actor)?, id, input, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(delete, path = "/api/v1/meal-templates/{id}", tag = "meals",
    params(("id" = String, Path, description = "template id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
) -> ApiResult<StatusCode> {
    manage(&auth.actor)?;
    s.services
        .meals
        .template_delete(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

#[utoipa::path(post, path = "/api/v1/meal-templates/{id}/preview", tag = "meals",
    request_body = TemplatePreviewReq,
    params(("id" = String, Path, description = "template id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = TemplatePreview), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn preview_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
    ApiJson(req): ApiJson<TemplatePreviewReq>,
) -> ApiResult<Json<TemplatePreview>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .template_preview(household(&auth.actor)?, id, req)
            .await?,
    ))
}

/// The range is at most 366 days.
#[utoipa::path(post, path = "/api/v1/meal-templates/from-plan", tag = "meals",
    request_body = TemplateFromPlanReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = MealTemplateView), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn template_from_plan(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<TemplateFromPlanReq>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .template_from_plan(household(&auth.actor)?, req)
        .await?;
    Ok(created_template(v))
}

#[utoipa::path(post, path = "/api/v1/meal-templates/{id}/duplicate", tag = "meals",
    request_body = TemplateDuplicateReq,
    params(("id" = String, Path, description = "template id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = MealTemplateView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn duplicate_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
    ApiJson(req): ApiJson<TemplateDuplicateReq>,
) -> ApiResult<Response> {
    manage(&auth.actor)?;
    let v = s
        .services
        .meals
        .template_duplicate(household(&auth.actor)?, id, req)
        .await?;
    Ok(created_template(v))
}

/// Slots in `add_slots` keep their entries; every other target slot is replaced. The result
/// carries a `change_id` to undo.
#[utoipa::path(post, path = "/api/v1/meal-templates/{id}/apply", tag = "meals",
    request_body = TemplateApplyReq,
    params(("id" = String, Path, description = "template id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = PlanChangeResult), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 422, body = Problem)))]
pub async fn apply_template(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealTemplateId>,
    ApiJson(req): ApiJson<TemplateApplyReq>,
) -> ApiResult<Json<PlanChangeResult>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .template_apply(household(&auth.actor)?, actor_ref(&auth.actor), id, req)
            .await?,
    ))
}

// ---- grocery ----

#[utoipa::path(get, path = "/api/v1/grocery", tag = "meals",
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GroceryView), (status = 403, body = Problem)))]
pub async fn get_grocery(State(s): State<AppState>, auth: Auth) -> ApiResult<Json<GroceryView>> {
    read(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .grocery_view(household(&auth.actor)?)
            .await?,
    ))
}

/// 201 when created, 200 when the client-generated `id` already exists (offline replay).
#[utoipa::path(post, path = "/api/v1/grocery/items", tag = "meals", request_body = GroceryAddReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 201, body = GroceryItemView), (status = 200, body = GroceryItemView),
              (status = 403, body = Problem), (status = 422, body = Problem)))]
pub async fn add_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<GroceryAddReq>,
) -> ApiResult<Response> {
    require(&auth.actor, Action::AddGroceryItem, &Resource::GroceryItem)?;
    let (v, created) = s
        .services
        .meals
        .grocery_add(household(&auth.actor)?, actor_ref(&auth.actor), req)
        .await?;
    let status = if created {
        StatusCode::CREATED
    } else {
        StatusCode::OK
    };
    let mut res = (status, Json(&v)).into_response();
    res.headers_mut().insert(ETAG, etag(v.version));
    Ok(res)
}

#[utoipa::path(post, path = "/api/v1/grocery/items/{id}/check", tag = "meals",
    params(("id" = String, Path, description = "item id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GroceryItemView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn check_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryItemId>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::CheckGroceryItem,
        &Resource::GroceryItem,
    )?;
    let v = s
        .services
        .meals
        .grocery_check(household(&auth.actor)?, actor_ref(&auth.actor), id)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(post, path = "/api/v1/grocery/items/{id}/uncheck", tag = "meals",
    params(("id" = String, Path, description = "item id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GroceryItemView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn uncheck_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryItemId>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::CheckGroceryItem,
        &Resource::GroceryItem,
    )?;
    let v = s
        .services
        .meals
        .grocery_uncheck(household(&auth.actor)?, id)
        .await?;
    Ok(with_etag(v.version, v))
}

/// Moves a staples-check item onto the list.
#[utoipa::path(post, path = "/api/v1/grocery/items/{id}/low", tag = "meals",
    params(("id" = String, Path, description = "item id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GroceryItemView), (status = 403, body = Problem),
              (status = 404, body = Problem)))]
pub async fn low_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryItemId>,
) -> ApiResult<Response> {
    require(
        &auth.actor,
        Action::CheckGroceryItem,
        &Resource::GroceryItem,
    )?;
    let v = s
        .services
        .meals
        .grocery_low(household(&auth.actor)?, id)
        .await?;
    Ok(with_etag(v.version, v))
}

#[utoipa::path(put, path = "/api/v1/grocery/items/{id}", tag = "meals", request_body = GroceryItemPatch,
    params(("id" = String, Path, description = "item id"),
           ("If-Match" = Option<String>, Header, description = "record version")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GroceryItemView), (status = 403, body = Problem),
              (status = 404, body = Problem), (status = 409, body = Problem),
              (status = 422, body = Problem), (status = 428, body = Problem)))]
pub async fn put_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryItemId>,
    headers: HeaderMap,
    ApiJson(patch): ApiJson<GroceryItemPatch>,
) -> ApiResult<Response> {
    require(&auth.actor, Action::EditGrocery, &Resource::GroceryItem)?;
    let expected = parse_version(&headers, None)?;
    let v = s
        .services
        .meals
        .grocery_update(household(&auth.actor)?, id, patch, expected)
        .await?;
    Ok(with_etag(v.version, v))
}

/// 404 is expected when an offline replay deletes an item that is already gone.
#[utoipa::path(delete, path = "/api/v1/grocery/items/{id}", tag = "meals",
    params(("id" = String, Path, description = "item id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem)))]
pub async fn delete_grocery_item(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<GroceryItemId>,
) -> ApiResult<StatusCode> {
    require(&auth.actor, Action::EditGrocery, &Resource::GroceryItem)?;
    s.services
        .meals
        .grocery_delete(household(&auth.actor)?, id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}

/// From `entry_ids`, or from the plan between `from` and `to`. Running it twice adds nothing.
#[utoipa::path(post, path = "/api/v1/grocery/generate", tag = "meals", request_body = GenerateReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = GenerateResult), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn generate_grocery(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<GenerateReq>,
) -> ApiResult<Json<GenerateResult>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .generate(household(&auth.actor)?, actor_ref(&auth.actor), req)
            .await?,
    ))
}

/// Recomputes the entries' contributions from the current plan.
#[utoipa::path(post, path = "/api/v1/grocery/reconcile", tag = "meals", request_body = ReconcileReq,
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 200, body = ReconcileResult), (status = 403, body = Problem),
              (status = 422, body = Problem)))]
pub async fn reconcile_grocery(
    State(s): State<AppState>,
    auth: Auth,
    ApiJson(req): ApiJson<ReconcileReq>,
) -> ApiResult<Json<ReconcileResult>> {
    manage(&auth.actor)?;
    Ok(Json(
        s.services
            .meals
            .reconcile(household(&auth.actor)?, actor_ref(&auth.actor), req)
            .await?,
    ))
}

/// An adult, or the member or device that made the change, within ten minutes.
#[utoipa::path(post, path = "/api/v1/meals/changes/{id}/undo", tag = "meals",
    params(("id" = String, Path, description = "change id")),
    security(("cookie" = []), ("bearer" = [])),
    responses((status = 204), (status = 403, body = Problem), (status = 404, body = Problem),
              (status = 409, body = Problem, description = "code `already_undone`"),
              (status = 410, body = Problem, description = "code `expired`")))]
pub async fn undo_meals_change(
    State(s): State<AppState>,
    auth: Auth,
    ApiPath(id): ApiPath<MealsChangeId>,
) -> ApiResult<StatusCode> {
    read(&auth.actor)?;
    s.services
        .meals
        .undo(household(&auth.actor)?, actor_ref(&auth.actor), id)
        .await?;
    Ok(StatusCode::NO_CONTENT)
}
