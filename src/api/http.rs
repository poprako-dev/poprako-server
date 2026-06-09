mod auth_token;
pub mod middleware;
mod openapi;

pub mod handler;
pub mod router;
pub mod server;

mod result {
    use std::num::NonZeroU16;

    use axum::Json;
    use axum::http::StatusCode;
    use axum::http::header::{HeaderMap, HeaderName, HeaderValue, SET_COOKIE};
    use axum::response::{IntoResponse, Response};
    use cookie::Cookie;
    use serde::Serialize;
    use utoipa::ToSchema;

    use poprako_util::i18n::trl;
    use poprako_util::rename::StdResult;

    use crate::domain::result::{DomainError, ExpectedVariant};
    use crate::usecase::result::UseCaseError;

    #[derive(Debug, Serialize, ToSchema)]
    pub struct HttpError {
        #[serde(skip)]
        status: StatusCode,

        #[schema(value_type = u16)]
        code: NonZeroU16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    }

    impl std::fmt::Display for HttpError {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(
                f,
                "HttpError(code={}, message={})",
                self.code,
                self.message.as_deref().unwrap_or("(no message)"),
            )
        }
    }

    impl HttpError {
        pub fn expected(variant: &ExpectedVariant, message: &str) -> Self {
            match variant {
                ExpectedVariant::Argument => Self {
                    status: StatusCode::BAD_REQUEST,
                    code: NonZeroU16::new(2).unwrap(),
                    message: Some(message.to_string()),
                },
                ExpectedVariant::Authentication => Self {
                    status: StatusCode::UNAUTHORIZED,
                    code: NonZeroU16::new(3).unwrap(),
                    message: Some(message.to_string()),
                },
                ExpectedVariant::Conflict => Self {
                    status: StatusCode::CONFLICT,
                    code: NonZeroU16::new(4).unwrap(),
                    message: Some(message.to_string()),
                },
            }
        }

        pub fn not_found() -> Self {
            Self {
                status: StatusCode::NOT_FOUND,
                code: NonZeroU16::new(5).unwrap(),
                message: None,
            }
        }

        pub fn internal(message: Option<String>) -> Self {
            Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: NonZeroU16::new(1).unwrap(), // placeholder
                message,
            }
        }
    }

    impl From<UseCaseError> for HttpError {
        fn from(err: UseCaseError) -> Self {
            match err.as_ref() {
                DomainError::Expected { variant, message } => {
                    tracing::debug!("[HttpError::from<UseCaseErr>] Expected error: {}", message);
                    Self::expected(variant, message)
                }
                DomainError::Unrecoverable { .. } => {
                    tracing::warn!("[HttpError::from<UseCaseErr>] Unrecoverable error concealed");
                    Self::internal(Some(trl("error-internal")))
                }
            }
        }
    }

    impl IntoResponse for HttpError {
        fn into_response(self) -> Response {
            (self.status, Json(self)).into_response()
        }
    }

    #[derive(Debug, Serialize, ToSchema)]
    pub struct HttpResponse<T> {
        /// Only for IntoResponse implementation, not serialized in response body.
        #[serde(skip)]
        status: StatusCode,

        #[serde(skip)]
        headers: HeaderMap,

        /// Business-level status code. Always 0 for successful response, non-zero for error response.
        code: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<T>,
    }

    impl<T> HttpResponse<T> {
        pub fn with_status(mut self, status: StatusCode) -> Self {
            self.status = status;
            self
        }
    }

    impl<T> From<T> for HttpResponse<T>
    where
        T: Serialize,
    {
        fn from(data: T) -> Self {
            Self {
                status: StatusCode::OK,
                headers: HeaderMap::new(),
                code: 0,
                message: None,
                data: Some(data),
            }
        }
    }

    impl<T> HttpResponse<T>
    where
        T: Serialize,
    {
        /// Adds a header to the response.
        pub fn with_header(mut self, name: &str, value: &str) -> Self {
            if let (Ok(name), Ok(value)) = (
                HeaderName::from_bytes(name.as_bytes()),
                HeaderValue::from_str(value),
            ) {
                self.headers.insert(name, value);
            }

            self
        }

        /// Sets a `Set-Cookie` header.
        pub fn with_cookie(mut self, cookie: &Cookie) -> Self {
            if let Ok(value) = HeaderValue::from_str(&cookie.to_string()) {
                self.headers.insert(SET_COOKIE, value);
            }

            self
        }
    }

    impl<T> IntoResponse for HttpResponse<T>
    where
        T: Serialize,
    {
        fn into_response(mut self) -> Response {
            let headers = std::mem::replace(&mut self.headers, HeaderMap::new());
            let mut response = (self.status, Json(self)).into_response();
            response.headers_mut().extend(headers);
            response
        }
    }

    pub type HttpResult<T> = StdResult<HttpResponse<T>, HttpError>;

    pub fn accept<T>(data: T, _status_code: StatusCode) -> HttpResult<T>
    where
        T: Serialize,
    {
        Ok(HttpResponse::from(data))
    }

    pub trait Accept {
        type Data: Serialize;

        /// A chainable method to convert a data object into an `HttpResult` with a 200 OK status.
        fn accept(self, status_code: StatusCode) -> HttpResult<Self::Data>;
    }

    impl<T> Accept for T
    where
        T: Serialize,
    {
        type Data = T;

        fn accept(self, status_code: StatusCode) -> HttpResult<Self::Data> {
            accept(self, status_code)
        }
    }
}
