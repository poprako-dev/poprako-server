use axum::Router;
use axum::middleware;
use axum::routing::post;

use crate::api::harness::Harness;
use crate::api::http::handler;
use crate::api::http::middleware::authorize;

pub fn new(harn: Harness) -> Router<Harness> {
    // Public auth routes — no authorization required.
    let auth_routes = Router::new().route(
        "/api/v1/auth/register",
        post(handler::authorization::sign_up_user),
    );

    // Protected routes — require a valid authorization token.
    let protected_routes = Router::new()
        // Future protected routes go here.
        .layer(middleware::from_fn_with_state(
            harn.clone(),
            authorize,
        ));

    auth_routes.merge(protected_routes)
}
