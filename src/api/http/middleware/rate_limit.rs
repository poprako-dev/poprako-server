//! Rate limiting middleware using the governor crate.
//!
//! Shares one configured token bucket across an application's routes.

#[cfg(test)]
mod tests;

use std::sync::Arc;

use axum::extract::{Request, State};
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

use crate::config::http::RateLimitConfig;

/// Constructs one shared limiter for an application instance.
#[must_use]
pub fn new(config: RateLimitConfig) -> Arc<DefaultDirectRateLimiter> {
    //
    Arc::new(RateLimiter::direct(
        Quota::per_second(config.requests_per_second).allow_burst(config.burst),
    ))
}

/// Enforces the application quota using explicitly injected shared state.
pub async fn rate_limit(
    State(limiter): State<Arc<DefaultDirectRateLimiter>>,
    request: Request,
    next: Next,
) -> Response {
    //
    if limiter.check().is_err() {
        //
        tracing::warn!(
            status = 429,
            uri = request.uri().path(),
            "rate limit exceeded"
        );

        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    next.run(request).await
}
