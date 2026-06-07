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
    ),
    components(schemas(
        crate::api::http::result::HttpError,
        crate::usecase::data_object::user::UserBase,
        crate::usecase::data_object::user::SignUpParams,
        crate::usecase::data_object::user::SignUpReply,
        crate::usecase::data_object::user::SignInParams,
        crate::usecase::data_object::user::SignInReply,
        crate::usecase::data_object::user::UserInfoUpdateParams,
        crate::usecase::data_object::user::ReserveAvatarParams,
        crate::usecase::data_object::user::ReserveAvatarReply,
    )),
    tags(
        (name = "health", description = "Health-check endpoints"),
        (name = "auth", description = "Authentication endpoints"),
        (name = "users", description = "User management endpoints"),
    )
)]
pub struct ApiDoc;
