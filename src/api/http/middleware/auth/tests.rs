#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// extract_token(extract_token)(positive): token extraction preserves cookie precedence, prefix handling, and whitespace rules.
// extract_token(extract_token)(negative): missing and empty credentials produce an empty token.

use axum::body::Body;

use super::*;

#[test]
fn extract_token_preserves_header_and_cookie_semantics() {
    //
    for (cookie, authorization, expected) in [
        (None, None, ""),
        (None, Some(""), ""),
        (None, Some("Bearer "), ""),
        (None, Some("Bearer header-token"), "header-token"),
        (None, Some("header-token"), "header-token"),
        (None, Some(" Bearer header-token "), " Bearer header-token "),
        (None, Some("Bearer header-token "), "header-token "),
        (
            Some("authorization-token=cookie-token"),
            None,
            "cookie-token",
        ),
        (
            Some("other=ignored; authorization-token = Bearer cookie-token \t"),
            Some("Bearer header-token"),
            "cookie-token",
        ),
        (
            Some("authorization-token= cookie-token "),
            Some("Bearer header-token"),
            "cookie-token",
        ),
        (
            Some("other=ignored"),
            Some("Bearer header-token"),
            "header-token",
        ),
        (
            Some("authorization-token="),
            Some("Bearer header-token"),
            "",
        ),
        (Some("authorization-token= Bearer "), None, "Bearer"),
        (
            Some("authorization-token=first; authorization-token=second"),
            None,
            "first",
        ),
    ] {
        //
        let mut builder = Request::builder();

        if let Some(cookie) = cookie {
            builder = builder.header(header::COOKIE, cookie);
        }

        if let Some(authorization) = authorization {
            builder = builder.header(header::AUTHORIZATION, authorization);
        }

        let request = builder.body(Body::empty()).unwrap();

        assert_eq!(extract_token(&request), expected);
    }
}
