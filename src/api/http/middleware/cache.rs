//! Prevents HTTP clients and shared caches from storing API responses.

#[cfg(test)]
mod tests;

use axum::http::HeaderValue;
use axum::http::header::CACHE_CONTROL;
use axum::response::Response;

/// Disables response storage, overriding any handler cache policy.
pub async fn disable_cache(mut response: Response) -> Response {
    //
    response
        .headers_mut()
        .insert(CACHE_CONTROL, HeaderValue::from_static("no-store"));

    response
}
