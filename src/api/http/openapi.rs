use utoipa::OpenApi;

use crate::api::http::handler;

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
        handler::team::list,
        handler::team::update_info,
        handler::team::reserve_avatar,
        handler::team::mark_avatar_uploaded,
        handler::team::delete,
        handler::member::create,
        handler::member::list_infos,
        handler::member::list_mine,
        handler::member::join,
        handler::member::list_my_members,
        handler::member::update_roles,
        handler::member::delete,
        handler::workset::create,
        handler::workset::list,
        handler::workset::update,
        handler::workset::delete,
    ),
    components(schemas(
        crate::api::http::result::HttpError,
        crate::usecase::data_object::user::UserBase,
        crate::usecase::data_object::user::SignUpParams,
        crate::usecase::data_object::user::SignUpReply,
        crate::usecase::data_object::user::SignInParams,
        crate::usecase::data_object::user::SignInReply,
        crate::usecase::data_object::user::UserInfoUpdateParams,
        crate::usecase::data_object::user::AvatarReserveParams,
        crate::usecase::data_object::user::AvatarReserveReply,
        crate::usecase::data_object::user::AvatarMarkUploadedParams,
        crate::usecase::data_object::team::TeamBase,
        crate::usecase::data_object::team::TeamCreateParams,
        crate::usecase::data_object::team::TeamInfoUpdateParams,
        crate::usecase::data_object::team::TeamAvatarReserveParams,
        crate::usecase::data_object::team::TeamAvatarReserveReply,
        crate::usecase::data_object::team::TeamAvatarMarkUploadedParams,
        crate::usecase::data_object::member::MemberBase,
        crate::usecase::data_object::member::MemberCreateParams,
        crate::usecase::data_object::member::MemberCreateReply,
        crate::usecase::data_object::member::MemberRoleUpdateParams,
        crate::usecase::data_object::member::MemberJoinParams,
        crate::usecase::data_object::workset::WorksetBase,
        crate::usecase::data_object::workset::WorksetCreateParams,
        crate::usecase::data_object::workset::WorksetCreateReply,
        crate::usecase::data_object::workset::WorksetUpdateParams,
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
