use axum::Router;
use axum::routing::post;

use crate::api::harness::Harness;
use crate::api::http::handler;

pub fn new() -> Router<Harness> {
    Router::new().route(
        "/api/v1/auth/register",
        post(handler::authorization::sign_up_user),
    )
}
