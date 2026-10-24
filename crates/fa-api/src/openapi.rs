use utoipa::openapi::security::{ApiKey, ApiKeyValue, HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

use crate::routes::{self, *};
use crate::schemas::*;

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
    ),
    components(schemas(
        Problem, Location, HouseholdDto, MemberPublicDto, MemberDto, DeviceDto, UserAccountDto,
        LoginReq, LoginResp, MeResponse, ActorView, CreateMemberReq, PatchMemberReq,
        CreateAccountReq, PasswordReq, PinReq, PairDeviceReq, PairDeviceResp, ElevateReq,
        ElevateResp, SessionView,
    )),
    modifiers(&Security)
)]
struct ApiDoc;

pub fn openapi() -> utoipa::openapi::OpenApi {
    ApiDoc::openapi()
}
