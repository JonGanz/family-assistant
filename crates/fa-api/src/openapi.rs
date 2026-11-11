use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::Problem;
use crate::feeds;
use crate::routes::{self, *};
use crate::{ai, mail, meals, notify, tasks};
use fa_domain::system_api::*;
use fa_domain::*;

struct Security;

impl Modify for Security {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let c = openapi.components.get_or_insert_with(Default::default);
        c.add_security_scheme(
            "cookie",
            SecurityScheme::ApiKey(ApiKey::Cookie(ApiKeyValue::new(crate::SESSION_COOKIE))),
        );
        c.add_security_scheme(
            "bearer",
            SecurityScheme::Http(HttpBuilder::new().scheme(HttpAuthScheme::Bearer).build()),
        );
    }
}

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Family Assistant API",
        version = "1",
        description = "Mutating requests must send `X-FA-Request: 1`. Errors are `application/problem+json`."
    ),
    paths(
        routes::healthz, routes::readyz, routes::login, routes::logout, routes::me,
        routes::list_members, routes::create_member, routes::patch_member,
        routes::create_account, routes::reset_password, routes::set_pin,
        routes::list_devices, routes::pair_device, routes::revoke_device, routes::elevate, routes::elevate_deprecated,
        routes::list_sessions, routes::revoke_session, routes::stream, routes::openapi_json,
        crate::setup::get_setup, crate::setup::setup_household, crate::setup::setup_owner,
        crate::setup::setup_complete, crate::system::master_key,
        crate::system::master_key_confirm, crate::system::get_settings,
        crate::system::patch_settings, crate::system::get_about,
        feeds::get_household, feeds::patch_household,
        feeds::put_avatar, feeds::delete_avatar, feeds::get_avatar,
        feeds::oauth_start, feeds::oauth_callback, feeds::list_oauth_accounts,
        feeds::list_sources, feeds::create_source, feeds::test_source, feeds::patch_source,
        feeds::delete_source, feeds::sync_source, feeds::list_calendars, feeds::patch_calendar,
        feeds::list_events, feeds::get_weather, feeds::list_advisories,
        feeds::list_rules, feeds::create_rule, feeds::patch_rule, feeds::delete_rule,
        feeds::validate_rule, feeds::get_device, feeds::patch_device, feeds::get_dashboard,
        feeds::get_status, feeds::get_failed_jobs, feeds::retry_failed_jobs,
        tasks::list_tasks, tasks::create_task, tasks::get_task, tasks::patch_task,
        tasks::delete_task, tasks::complete_task, tasks::reopen_task, tasks::skip_task,
        tasks::accept_task, tasks::claim_task, tasks::release_task, tasks::tick_task_step,
        tasks::list_task_lists, tasks::create_task_list, tasks::patch_task_list,
        tasks::delete_task_list, tasks::list_definitions, tasks::create_definition,
        tasks::patch_definition, tasks::delete_definition, tasks::validate_definition,
        tasks::list_routines, tasks::create_routine, tasks::patch_routine,
        tasks::delete_routine, tasks::routines_today, tasks::tick_routine_step,
        tasks::list_away, tasks::add_away, tasks::delete_member_away, tasks::delete_away,
        notify::get_prefs, notify::put_prefs, notify::test_prefs,
        mail::list_mail_sources, mail::create_mail_source, mail::patch_mail_source,
        mail::delete_mail_source, mail::sync_mail_source, mail::test_imap,
        mail::list_messages, mail::get_message, mail::label_message, mail::set_eval_sample,
        mail::message_action, mail::undo_mail_action, mail::message_task, mail::labelling_next,
        mail::list_orgs, mail::create_org, mail::patch_org, mail::delete_org,
        mail::add_org_contact, mail::patch_org_contact, mail::delete_org_contact,
        mail::list_enrollments, mail::create_enrollment, mail::patch_enrollment,
        mail::delete_enrollment, mail::list_mail_rules, mail::create_mail_rule,
        mail::patch_mail_rule, mail::delete_mail_rule, mail::test_mail_rule,
        mail::list_mail_signals, mail::patch_mail_signal, mail::list_categories,
        mail::create_category, mail::patch_category, mail::get_inbox, mail::approve_inbox,
        mail::reject_inbox, mail::discard_inbox, mail::discard_past_events, mail::list_bills, mail::patch_bill,
        ai::get_endpoint, ai::put_endpoint, ai::test_endpoint, ai::list_models,
        ai::get_settings, ai::put_settings, ai::get_pipeline, ai::put_pipeline,
        ai::get_stats, ai::get_backlog, ai::process_backlog, ai::extract_recipe,
        meals::list_recipes, meals::list_recipe_tags, meals::create_recipe, meals::get_recipe, meals::put_recipe, meals::delete_recipe, meals::put_favourite, meals::put_rating, meals::get_recipe_image, meals::post_recipe_image, meals::delete_recipe_image, meals::import_recipe, meals::match_recipe_line, meals::list_ingredients, meals::create_ingredient, meals::put_ingredient, meals::delete_ingredient, meals::merge_ingredient, meals::get_meals_settings, meals::put_meals_settings, meals::get_meal_slots, meals::put_meal_slots, meals::get_meal_aisles, meals::put_meal_aisles, meals::get_meal_stores, meals::put_meal_stores, meals::get_store_aisles, meals::put_store_aisles, meals::get_meal_plan, meals::add_plan_entry, meals::put_plan_entry, meals::delete_plan_entry, meals::swap_plan_entries, meals::list_templates, meals::create_template, meals::get_template, meals::put_template, meals::delete_template, meals::preview_template, meals::template_from_plan, meals::duplicate_template, meals::apply_template, meals::get_grocery, meals::add_grocery_item, meals::check_grocery_item, meals::uncheck_grocery_item, meals::low_grocery_item, meals::put_grocery_item, meals::delete_grocery_item, meals::generate_grocery, meals::reconcile_grocery, meals::undo_meals_change,
    ),
    components(schemas(
        crate::tasks::ViewParam, crate::tasks::StatusParam,
        crate::mail::DiscardedCount, EventApproval, TaskApproval, ContactApproval, BillPatch, BillPatchStatus, InboxItem,
        crate::feeds::PurposeParam, crate::feeds::SourceViewExt, crate::feeds::SourcePatchExt,
        crate::feeds::CalendarViewExt, crate::feeds::CalendarPatchExt,
        RecipeSummary, RecipePage, RecipeView, RecipeLineView, IngredientProposal, RecipeInput, MemberRating, RatingsView, RecipeImportReq, LineMatchReq, IngredientView, IngredientInput, IngredientMergeReq, MealsSettingsView, MealsSettingsPatch, MealSlotView, AisleView, StoreView, StoreAisleView, MealPlanEntryView, MealPlanEntryInput, MealPlanEntryPatch, SwapReq, GroceryImpact, PlanChangeResult, MealTemplateSummary, TemplateCellView, MealTemplateView, TemplateCellInput, MealTemplateInput, TemplateFromPlanReq, TemplateDuplicateReq, TemplatePreviewReq, SlotRef, TemplateApplyReq, TemplateTarget, TemplatePreview, SourceHint, GroceryItemView, StapleView, GroceryView, GroceryAddReq, GroceryItemPatch, GenerateReq, GenerateResult, ReconcileReq, ReconcileResult, DashboardMeal,
        meals::FavouriteReq, meals::RatingReq,
        Problem, MemberView, MemberPublic, MemberFull, DashboardSettings,
        LoginReq, LoginResp, MeResponse, ActorView, CreateMemberReq, PatchMemberReq,
        CreateAccountReq, PasswordReq, PinReq, PairDeviceReq, PairDeviceResp, ElevateReq,
        ElevateResp, SessionView, PasswordBody, OwnPasswordChange, crate::feeds::HouseholdViewExt,
        crate::setup::SetupHouseholdCreated, SetupStatus, NewSetupHousehold, SetupLocation,
        NewSetupOwner, fa_domain::theme::Hue, fa_domain::theme::ThemeChoice, fa_domain::theme::DarkTheme, fa_domain::theme::ThemeMode, MasterKeyRequest, MasterKeyView, SettingSource, SettingKind, SettingView,
        ConnectionInfo, AboutView, SystemSettingsView, SystemSettingsPatch,
    )),
    modifiers(&Security)
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
