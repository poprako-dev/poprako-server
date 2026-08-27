//! Cross-origin browser access for the production web client.

#[cfg(test)]
mod tests;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE, ORIGIN};
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// Builds the CORS layer for requests from the `PopRaKo` web client.
pub fn cors() -> CorsLayer {
    //
    let origin = HeaderValue::try_from("https://poprako.com")
        .unwrap_or_else(|_| HeaderValue::from_name(ORIGIN));

    CorsLayer::new()
        .allow_origin(origin)
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
