use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::error::Problem;
use crate::feeds;
use crate::routes::{self, *};
use crate::{mail, notify, tasks};
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
        routes::list_devices, routes::pair_device, routes::revoke_device, routes::elevate,
        routes::list_sessions, routes::revoke_session, routes::stream, routes::openapi_json,
        crate::setup::get_setup, crate::setup::setup_household, crate::setup::setup_owner,
        crate::setup::setup_complete, crate::system::master_key,
        crate::system::master_key_confirm, crate::system::get_settings,
        crate::system::patch_settings, crate::config::import_config,
        crate::config::export_config,
        feeds::get_household, feeds::patch_household,
        feeds::put_avatar, feeds::delete_avatar, feeds::get_avatar,
        feeds::oauth_start, feeds::oauth_callback, feeds::list_oauth_accounts,
        feeds::list_sources, feeds::create_source, feeds::test_source, feeds::patch_source,
        feeds::delete_source, feeds::sync_source, feeds::list_calendars, feeds::patch_calendar,
        feeds::list_events, feeds::get_weather, feeds::list_advisories,
        feeds::list_rules, feeds::create_rule, feeds::patch_rule, feeds::delete_rule,
        feeds::validate_rule, feeds::get_device, feeds::patch_device, feeds::get_dashboard,
        feeds::get_status,
        tasks::list_tasks, tasks::create_task, tasks::get_task, tasks::patch_task,
        tasks::delete_task, tasks::complete_task, tasks::reopen_task, tasks::skip_task,
        tasks::accept_task, tasks::claim_task, tasks::release_task, tasks::tick_task_step,
        tasks::list_task_lists, tasks::create_task_list, tasks::patch_task_list,
        tasks::delete_task_list, tasks::list_definitions, tasks::create_definition,
        tasks::patch_definition, tasks::delete_definition, tasks::validate_definition,
        tasks::list_routines, tasks::create_routine, tasks::patch_routine,
        tasks::delete_routine, tasks::routines_today, tasks::tick_routine_step,
        tasks::list_away, tasks::add_away, tasks::delete_away,
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
        mail::reject_inbox, mail::list_bills, mail::patch_bill,
    ),
    components(schemas(
        crate::tasks::ViewParam, crate::tasks::StatusParam,
        EventApproval, TaskApproval, ContactApproval, BillPatch, BillPatchStatus, InboxItem,
        crate::feeds::PurposeParam, crate::feeds::SourceViewExt, crate::feeds::SourcePatchExt,
        crate::feeds::CalendarViewExt, crate::feeds::CalendarPatchExt,
        Problem, MemberView, MemberPublic, MemberFull, DashboardSettings,
        LoginReq, LoginResp, MeResponse, ActorView, CreateMemberReq, PatchMemberReq,
        CreateAccountReq, PasswordReq, PinReq, PairDeviceReq, PairDeviceResp, ElevateReq,
        ElevateResp, SessionView, PasswordBody, OwnPasswordChange, crate::feeds::HouseholdViewExt,
        crate::setup::SetupHouseholdCreated, SetupStatus, NewSetupHousehold, SetupLocation,
        NewSetupOwner, MasterKeyRequest, MasterKeyView, SettingSource, SettingKind, SettingView,
        ConnectionInfo, SystemSettingsView, SystemSettingsPatch, fa_config::ImportReport,
    )),
    modifiers(&Security)
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
