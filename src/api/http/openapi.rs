use utoipa::OpenApi;

use crate::api::http::handler;
use crate::api::http::result::HttpError;
use crate::usecase::data_object;

/// Top-level OpenAPI documentation for the PopRaKo-R HTTP API.
///
/// All v1 endpoints are nested under `/api/v1` via axum's `Router::nest`.
/// The handler-level `#[utoipa::path]` attributes specify paths relative
/// to that nest point.
#[derive(OpenApi)]
#[openapi(
    paths(
        handler::health::check_health,
        handler::authorization::sign_up,
        handler::authorization::sign_in,
        handler::user::get_info,
        handler::user::get_my_info,
        handler::user::update_info,
        handler::user::reserve_avatar,
        handler::user::mark_avatar_uploaded,
        handler::team::create,
        handler::team::get_info,
        handler::team::list_infos,
        handler::team::update_info,
        handler::team::reserve_avatar,
        handler::team::mark_avatar_uploaded,
        handler::team::delete,
        handler::member::create,
        handler::member::list_infos,
        handler::member::list_my_infos,
        handler::member::join,
        handler::member::update_roles,
        handler::member::delete,
        handler::workset::create,
        handler::workset::list_infos,
        handler::workset::update_infos,
        handler::workset::delete,
    ),
    components(schemas(
        HttpError,
        data_object::user::UserInfo,
        data_object::user::SignUpParams,
        data_object::user::SignUpReply,
        data_object::user::SignInParams,
        data_object::user::SignInReply,
        data_object::user::InfoUpdateParams,
        data_object::user::AvatarReserveParams,
        data_object::user::AvatarReserveReply,
        data_object::user::AvatarMarkUploadedParams,
        data_object::team::TeamInfo,
        data_object::team::CreateParams,
        data_object::team::InfoUpdateParams,
        data_object::team::AvatarReserveParams,
        data_object::team::AvatarReserveReply,
        data_object::team::AvatarMarkUploadedParams,
        data_object::member::MemberInfo,
        data_object::member::CreateParams,
        data_object::member::CreateReply,
        data_object::member::RoleUpdateParams,
        data_object::member::JoinParams,
        data_object::workset::WorksetInfo,
        data_object::workset::WorksetCreateParams,
        data_object::workset::WorksetCreateReply,
        data_object::workset::WorksetUpdateParams,
    )),
    tags(
        (name = "health", description = "Health-check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
        (name = "teams", description = "Team management endpoints"),
        (name = "members", description = "Member management endpoints"),
        (name = "worksets", description = "Workset management endpoints"),
    )
)]
pub struct ApiDoc;
