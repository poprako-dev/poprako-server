use axum::Router;
use axum::routing::post;

use crate::api::http::AuthorizeLayer;
use crate::api::http::IdTraceLayer;
use crate::api::http::handler::authorization;
use crate::harness::Harness;

pub fn new(harn: Harness) -> Router<Harness> {
    // Public auth routes — no authorization required.
    let authorize_routes =
        Router::new().route("/api/v1/auth/register", post(authorization::sign_up_user));

    // Protected routes — require a valid authorization token.
    let protected_routes = Router::new().layer(AuthorizeLayer::new(harn));

    // Wrap with request-id + tracing middleware (outermost layer).
    authorize_routes
        .merge(protected_routes)
        .layer(IdTraceLayer::new())
}
