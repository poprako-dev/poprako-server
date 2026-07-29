//! HTTP boundary result types: success envelope, error envelope, and the
//! `Accept` trait that turns a usecase value into a valued response.
//!
//! Valued success responses serialize as [`HttpBody<T>`], the standard JSON
//! envelope containing `code` and `data`. Empty success responses use
//! [`NoContent`] to emit a `204 No Content` with no body. Errors are propagated
//! as [`HttpError`].

use std::num::NonZeroU16;

use axum::Json;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderValue, SET_COOKIE};
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use serde::Serialize;

use poprako_util::i18n::trl;
use poprako_util::rename::StdResult;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use crate::result::{Error as RegularError, ExpectedVariant};

#[cfg(test)]
// HTTP result test fixtures are compiled only for tests.
mod tests;

/// Business-level error envelope returned by all failing endpoints.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct HttpError {
    //
    /// HTTP status code returned to the client (e.g. 404, 500).
    #[serde(skip)]
    #[cfg_attr(feature = "swagger", schema(ignore))]
    status: StatusCode,

    /// Business error code identifying the failure reason.
    #[cfg_attr(feature = "swagger", schema(value_type = u16))]
    code: NonZeroU16,

    /// Human-readable error detail, omitted when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    #[cfg_attr(feature = "swagger", schema(ignore))]
    message: Option<String>,
}

impl HttpError {
    /// Builds an error from an expected application variant and message.
    pub fn expected(variant: ExpectedVariant, message: &str) -> Self {
        //
        let (status, code) = match variant {
            //
            ExpectedVariant::Args => (StatusCode::UNPROCESSABLE_ENTITY, 2),

            ExpectedVariant::Auth => (StatusCode::UNAUTHORIZED, 3),

            ExpectedVariant::Perm => (StatusCode::FORBIDDEN, 4),
        };

        Self {
            status,
            code: NonZeroU16::new(code).expect("non-zero error code"),
            message: Some(message.to_string()),
        }
    }

    /// `422 Unprocessable Entity` used for path/body id mismatch.
    pub fn unprocessable(message: &str) -> Self {
        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: NonZeroU16::new(7).expect("non-zero error code"),
            message: Some(message.to_string()),
        }
    }

    /// `500 Internal Server Error` concealing unrecoverable details.
    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: NonZeroU16::new(1).expect("non-zero error code"),
            message: Some(trl("error-internal")),
        }
    }
}

impl std::fmt::Display for HttpError {
    // Formats one failure as `HttpError(code=..., message=...)` for logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "HttpError(code={}, message={})",
            self.code,
            self.message.as_deref().unwrap_or("(no message)"),
        )
    }
}

impl From<RegularError> for HttpError {
    // Maps the shared result error to HTTP boundary error payload.
    fn from(source: RegularError) -> Self {
        match source {
            //
            RegularError::Expected { variant, message } => {
                //
                tracing::debug!(
                    "[HttpError::from] expected error: {}",
                    message
                );

                Self::expected(variant, &message)
            }

            RegularError::Unrecoverable { message } => {
                //
                tracing::warn!(
                    "[HttpError::from] unrecoverable error concealed: {}",
                    message
                );

                Self::internal()
            }
        }
    }
}

impl IntoResponse for HttpError {
    // Converts a boundary error into JSON response with status code.
    fn into_response(self) -> Response {
        (self.status, Json(self)).into_response()
    }
}

/// Success response body wrapping a usecase value.
///
/// Serializes as the standard JSON success envelope. HTTP metadata such as
/// status code, extra headers, and `Set-Cookie` values are not part of the JSON
/// body.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct HttpBody<T> {
    //
    /// HTTP status code set on the response (e.g. 200, 201).
    #[serde(skip)]
    #[cfg_attr(feature = "swagger", schema(ignore))]
    status: StatusCode,

    /// Extra HTTP headers merged into the response (e.g. Set-Cookie).
    #[serde(skip)]
    #[cfg_attr(feature = "swagger", schema(ignore))]
    headers: HeaderMap,

    /// Application-level success code, 0 for normal success.
    #[cfg_attr(feature = "swagger", schema(value_type = u16, example = 0))]
    code: u16,

    /// Business payload returned to the client.
    data: T,
}

impl<T> HttpBody<T> {
    /// Creates a valued success body with the given status.
    pub fn new(status: StatusCode, data: T) -> Self {
        Self {
            status,
            headers: HeaderMap::new(),
            code: 0,
            data,
        }
    }

    /// Appends a `Set-Cookie` header.
    pub fn with_cookie(mut self, cookie: &Cookie) -> Self {
        //
        if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
            self.headers.insert(SET_COOKIE, value);
        }

        self
    }
}

impl<T> IntoResponse for HttpBody<T>
where
    T: Serialize,
{
    // Converts a success body into JSON response and merges any recorded headers.
    fn into_response(self) -> Response {
        //
        let status = self.status;

        let headers = self.headers.clone();

        let mut response = (status, Json(self)).into_response();

        response.headers_mut().extend(headers);

        response
    }
}

/// Empty success marker emitted as `204 No Content` with no body.
///
/// Carries optional headers (e.g. `Set-Cookie`) that are merged into the
/// final response.
pub struct NoContent {
    /// Extra HTTP headers merged into the 204 response.
    headers: HeaderMap,
}

impl NoContent {
    /// Creates an empty `204 No Content` response with no extra headers.
    pub fn new() -> Self {
        Self {
            headers: HeaderMap::new(),
        }
    }

    /// Appends a `Set-Cookie` header to the response.
    pub fn with_cookie(mut self, cookie: &Cookie) -> Self {
        //
        if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
            self.headers.insert(SET_COOKIE, value);
        }

        self
    }
}

impl Default for NoContent {
    // Builds a default `204 No Content` marker.
    fn default() -> Self {
        Self::new()
    }
}

impl IntoResponse for NoContent {
    // Converts a 204 marker into an empty HTTP response with headers.
    fn into_response(self) -> Response {
        //
        let mut response = StatusCode::NO_CONTENT.into_response();

        response.headers_mut().extend(self.headers);

        response
    }
}

/// Result of a valued success response.
pub type HttpResult<T> = StdResult<HttpBody<T>, HttpError>;

/// Result of an empty success response (`204 No Content`).
pub type HttpNoContent = StdResult<NoContent, HttpError>;

/// Converts a usecase value into a valued [`HttpResult`] with the given status.
pub fn accept<T>(data: T, status_code: StatusCode) -> HttpResult<T>
where
    T: Serialize,
{
    Ok(HttpBody::new(status_code, data))
}

/// Trait form of [`accept`] so handlers can write `value.accept(StatusCode::OK)`.
pub trait Accept {
    /// The serializable response payload type returned by [`accept`].
    type Data: Serialize;

    /// Wraps `self` into an [`HttpResult`] carrying the provided status code.
    fn accept(self, status_code: StatusCode) -> HttpResult<Self::Data>;
}

impl<T> Accept for T
where
    T: Serialize,
{
    // Concrete payload type emitted by [`accept`] for this implementation.
    type Data = T;

    // Wraps a success body with the provided status code.
    fn accept(self, status_code: StatusCode) -> HttpResult<Self::Data> {
        accept(self, status_code)
    }
}

/// Returns a `204 No Content` result with an empty body.
pub fn no_content() -> StdResult<NoContent, HttpError> {
    Ok(NoContent::new())
}
