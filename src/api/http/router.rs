use axum::Router;
use axum::routing::{get, post, put};

use crate::api::http::AuthorizeLayer;
use crate::api::http::IdTraceLayer;
use crate::api::http::handler::authorization;
use crate::api::http::handler::user;
use crate::harness::Harness;

pub fn new(harn: Harness) -> Router<Harness> {
    // Public auth routes — no authorization required.
    let public_routes = Router::new()
        .route("/api/v1/auth/register", post(authorization::sign_up_user))
        .route("/api/v1/user/login", post(user::sign_in));

    // Protected routes — require a valid authorization token.
    let protected_routes = Router::new()
        .route("/api/v1/user/info", get(user::get_info))
        .route("/api/v1/user", put(user::update_info))
        .route("/api/v1/user/avatar/reserve", post(user::reserve_avatar))
        .route(
            "/api/v1/user/avatar/uploaded",
            post(user::mark_avatar_uploaded),
        )
        .layer(AuthorizeLayer::new(harn));

    // Wrap with request-id + tracing middleware (outermost layer).
    public_routes
        .merge(protected_routes)
        .layer(IdTraceLayer::new())
}
