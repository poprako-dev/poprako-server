//! Rate limiting middleware using the governor crate.
//!
//! Applies a token-bucket rate limiter with burst = 80 and average = 20
//! requests per second. Returns 429 Too Many Requests when exceeded.

use std::num::NonZeroU32;
use std::sync::OnceLock;

use axum::extract::Request;
use axum::http::StatusCode;
use axum::middleware::Next;
use axum::response::{IntoResponse as _, Response};
use governor::{DefaultDirectRateLimiter, Quota, RateLimiter};

/// `from_fn` handler that enforces a global rate limit.
pub async fn rate_limit(request: Request, next: Next) -> Response {
    //
    if limiter().check().is_err() {
        //
        tracing::warn!(uri = %request.uri(), "rate limit exceeded");

        return StatusCode::TOO_MANY_REQUESTS.into_response();
    }

    next.run(request).await
}

// Returns the global singleton rate limiter, created lazily at first call.
fn limiter() -> &'static DefaultDirectRateLimiter {
    // Lazily initialized rate-limiter singleton.
    static LIMITER: OnceLock<DefaultDirectRateLimiter> = OnceLock::new();

    LIMITER.get_or_init(|| {
        //
        RateLimiter::direct(
            Quota::per_second(NonZeroU32::new(20).unwrap())
                .allow_burst(NonZeroU32::new(80).unwrap()),
        )
    })
}
