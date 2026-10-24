//! HTTP boundary result types: success envelope, error envelope, and the
//! `Accept` trait that turns a usecase value into a valued response.
//!
//! Valued success responses serialize as [`HttpBody<T>`], the standard JSON
//! envelope containing `code` and `data`. Empty success responses use
//! [`NoContent`] to emit a `204 No Content` with no body. Errors are propagated
//! as [`HttpError`].

#[cfg(test)]
// HTTP result test fixtures are compiled only for tests.
mod tests;

use std::num::NonZeroU16;
use std::result::Result;

use axum::Json;
use axum::http::StatusCode;
use axum::http::header::{HeaderMap, HeaderValue, SET_COOKIE};
use axum::response::{IntoResponse, Response};
use cookie::Cookie;
use serde::ser::SerializeStruct as _;
use serde::{Serialize, Serializer};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

use poprako_util::i18n::trl;

use crate::result::{BaseError, ExpectedVariant};

/// Business-level error envelope returned by all failing endpoints.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct HttpError {
    /// HTTP status code returned to the client (e.g. 404, 500).
    #[serde(skip)]
    #[cfg_attr(feature = "swagger", schema(ignore))]
    status: StatusCode,

    /// Business error code identifying the failure reason.
    #[cfg_attr(feature = "swagger", schema(value_type = u16))]
    code: NonZeroU16,

    /// Human-readable error detail, omitted when absent.
    #[serde(rename = "message", skip_serializing_if = "Option::is_none")]
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
            code: nonzero_code(code),
            message: Some(message.to_string()),
        }
    }

    /// `422 Unprocessable Entity` used for path/body id mismatch.
    pub fn unprocessable(message: &str) -> Self {
        //
        tracing::warn!(
            status = 422,
            error_variant = "PathBodyMismatch",
            error_message = message,
            "HTTP argument error",
        );

        Self {
            status: StatusCode::UNPROCESSABLE_ENTITY,
            code: nonzero_code(7),
            message: Some(message.to_string()),
        }
    }

    /// `500 Internal Server Error` concealing unrecoverable details.
    pub fn internal() -> Self {
        Self {
            status: StatusCode::INTERNAL_SERVER_ERROR,
            code: nonzero_code(1),
            message: Some(trl("error-internal")),
        }
    }

    /// 503 Service Unavailable with a generic localized message.
    pub fn unavailable() -> Self {
        Self {
            status: StatusCode::SERVICE_UNAVAILABLE,
            code: nonzero_code(9),
            message: Some(trl("error-unavailable")),
        }
    }
}

impl std::fmt::Display for HttpError {
    // Formats one failure as `HttpError(code=..., message=...)` for logs.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        //
        write!(
            f,
            "HttpError(code={}, message={})",
            self.code,
            self.message.as_deref().unwrap_or("(no message)"),
        )
    }
}

impl From<BaseError> for HttpError {
    // Maps the shared result error to HTTP boundary error payload.
    fn from(source: BaseError) -> Self {
        //
        match source {
            //
            BaseError::Expected { variant, message } => {
                Self::expected(variant, &message)
            }

            BaseError::Retryable { message } => Self {
                status: StatusCode::CONFLICT,
                code: nonzero_code(8),
                message: Some(message),
            },

            BaseError::Unavailable { .. } => Self::unavailable(),

            BaseError::Unrecoverable { .. } => Self::internal(),
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
pub struct HttpBody<T> {
    /// HTTP status code set on the response (e.g. 200, 201).
    status: StatusCode,

    /// Extra HTTP headers merged into the response (e.g. Set-Cookie).
    headers: HeaderMap,

    /// Application-level success code, 0 for normal success.
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

#[cfg(feature = "swagger")]
impl<T> utoipa::__dev::ComposeSchema for HttpBody<T>
where
    T: ToSchema,
{
    // Builds the same envelope schema while composing the requested payload schema.
    fn compose(
        generics: Vec<utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>>,
    ) -> utoipa::openapi::RefOr<utoipa::openapi::schema::Schema> {
        //
        let data_schema = generics.first().cloned().unwrap_or_else(T::schema);

        let mut code_schema = utoipa::openapi::schema::ObjectBuilder::new()
            .schema_type(utoipa::openapi::schema::Type::Integer)
            .format(Some(utoipa::openapi::schema::SchemaFormat::KnownFormat(
                utoipa::openapi::schema::KnownFormat::Int32,
            )))
            .description(Some(
                "Application-level success code, 0 for normal success.",
            ))
            .minimum(Some(0))
            .build();

        code_schema.example = Some(0.into());

        utoipa::openapi::schema::ObjectBuilder::new()
            .description(Some(concat!(
                "Success response body wrapping a usecase value.\n\n",
                "Serializes as the standard JSON success envelope. ",
                "HTTP metadata such as\n",
                "status code, extra headers, and `Set-Cookie` values ",
                "are not part of the JSON\n",
                "body.",
            )))
            .property("code", code_schema)
            .property("data", data_schema)
            .required("code")
            .required("data")
            .into()
    }
}

#[cfg(feature = "swagger")]
impl<T> ToSchema for HttpBody<T>
where
    T: ToSchema,
{
    // Collects the payload's nested component schemas.
    fn schemas(
        schemas: &mut Vec<(
            String,
            utoipa::openapi::RefOr<utoipa::openapi::schema::Schema>,
        )>,
    ) {
        T::schemas(schemas);
    }

    // Keeps the generic envelope's established schema name.
    fn name() -> std::borrow::Cow<'static, str> {
        std::borrow::Cow::Borrowed("HttpBody")
    }
}

impl<T> Serialize for HttpBody<T>
where
    T: Serialize,
{
    // Serializes only the stable application envelope fields.
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        //
        let mut body = serializer.serialize_struct("HttpBody", 2)?;

        body.serialize_field("code", &self.code)?;

        body.serialize_field("data", &self.data)?;

        body.end()
    }
}

impl<T> IntoResponse for HttpBody<T>
where
    T: Serialize,
{
    // Converts a success body into JSON response and merges any recorded headers.
    fn into_response(self) -> Response {
        //
        let Self {
            status,
            headers,
            code,
            data,
        } = self;

        let body = Self {
            status,
            headers: HeaderMap::new(),
            code,
            data,
        };

        let mut response = (status, Json(body)).into_response();

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
pub type HttpResult<T> = Result<HttpBody<T>, HttpError>;

/// Result of an empty success response (`204 No Content`).
pub type HttpNoContent = Result<NoContent, HttpError>;

/// Converts a usecase value into a valued success body.
pub fn accept<T>(data: T, status_code: StatusCode) -> HttpBody<T>
where
    T: Serialize,
{
    HttpBody::new(status_code, data)
}

/// Trait form of [`accept`] so handlers can write `value.accept(StatusCode::OK)`.
pub trait Accept {
    /// The serializable response payload type returned by [`accept`].
    type Data: Serialize;

    /// Wraps `self` into an [`HttpBody`] carrying the provided status code.
    fn accept(self, status_code: StatusCode) -> HttpBody<Self::Data>;
}

impl<T> Accept for T
where
    T: Serialize,
{
    // Concrete payload type emitted by [`accept`] for this implementation.
    type Data = T;

    // Wraps a success body with the provided status code.
    fn accept(self, status_code: StatusCode) -> HttpBody<Self::Data> {
        accept(self, status_code)
    }
}

/// Returns a `204 No Content` response with an empty body.
pub fn no_content() -> NoContent {
    NoContent::new()
}

// Converts one HTTP application code into its non-zero representation.
const fn nonzero_code(code: u16) -> NonZeroU16 {
    //
    match NonZeroU16::new(code) {
        //
        Some(code) => code,

        None => NonZeroU16::MIN,
    }
}
