//! Cookie and bearer-token constants used by the auth middleware and the
//! register/login handlers.

/// Name of the `HttpOnly` cookie carrying the bearer token.
pub const AUTH_COOKIE_NAME: &str = "authorization-token";

/// Prefix stripped from the `Authorization` header value before verification.
pub const AUTH_BEARER_PREFIX: &str = "Bearer ";
