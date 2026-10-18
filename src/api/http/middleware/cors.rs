//! Cross-origin browser access for the production web client.

#[cfg(test)]
mod tests;

use axum::http::header::{AUTHORIZATION, CONTENT_TYPE};
use axum::http::{HeaderValue, Method};
use tower_http::cors::CorsLayer;

/// Builds the CORS layer from the configured, validated web-client origin.
pub fn cors(origin: HeaderValue) -> CorsLayer {
    //
    CorsLayer::new()
        .allow_origin([origin])
        .allow_methods([
            Method::GET,
            Method::POST,
            Method::PUT,
            Method::PATCH,
            Method::DELETE,
        ])
        .allow_headers([AUTHORIZATION, CONTENT_TYPE])
}
