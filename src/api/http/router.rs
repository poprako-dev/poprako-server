use axum::Router;
use axum::routing::{get, post};

use crate::api::http::AuthorizeLayer;
use crate::api::http::IdTraceLayer;
use crate::api::http::handler::authorization;
use crate::api::http::handler::health;
use crate::api::http::handler::user;
use crate::api::http::openapi::ApiDoc;
use crate::harness::Harness;

pub fn new(harn: Harness) -> Router<Harness> {
    // Public auth routes — no authorization required.
    let v1_public = Router::new()
        .route("/check-health", get(health::check_health))
        .route("/auth/sign-up", post(authorization::sign_up))
        .route("/auth/sign-in", post(authorization::sign_in));

    // Protected routes — require a valid authorization token.
    let v1_protected = Router::new()
        .route("/users/{user_id}", get(user::get_info))
        .route("/users/me", get(user::get_my_info).put(user::update_info))
        .route(
            "/users/{user_id}/avatar/reserve",
            post(user::reserve_avatar),
        )
        .route(
            "/users/{user_id}/avatar/mark-uploaded",
            post(user::mark_avatar_uploaded),
        )
        .layer(AuthorizeLayer::new(harn));

    // Nest all v1 routes under /api/v1, wrapped with request-id + tracing.
    let mut router = Router::new()
        .nest("/api/v1", v1_public.merge(v1_protected))
        .layer(IdTraceLayer::new());

    // Swagger UI — only available in debug builds.
    #[cfg(debug_assertions)]
    {
        use utoipa::OpenApi as _;
        use utoipa_swagger_ui::SwaggerUi;

        router = router
            .merge(SwaggerUi::new("/api/swagger-ui").url("/api/openapi.json", ApiDoc::openapi()));
    }

    router
}
