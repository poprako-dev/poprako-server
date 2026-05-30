pub mod authorization;
pub mod user;

pub use result::{HttpError, HttpResult};

pub mod result {
    use axum::Json;
    use axum::extract::rejection::JsonRejection;
    use axum::http::StatusCode;
    use axum::http::header::{HeaderMap, HeaderName, HeaderValue};
    use axum::response::{IntoResponse, Response};
    use cookie::Cookie;
    use serde::Serialize;
    use utoipa::ToSchema;

    use crate::domain::result::{DomainError, ExpectedVariant};
    use crate::usecase::result::UseCaseError;
    use crate::util::i18n::trl;
    use crate::util::rename::StdResult;

    #[derive(Debug, Serialize, ToSchema)]
    pub struct HttpError {
        #[serde(skip)]
        status: StatusCode,

        code: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    }

    impl HttpError {
        pub fn expected(variant: &ExpectedVariant, message: &str) -> Self {
            match variant {
                ExpectedVariant::Argument => Self {
                    status: StatusCode::BAD_REQUEST,
                    code: 2,
                    message: Some(message.to_string()),
                },
                ExpectedVariant::Authentication => Self {
                    status: StatusCode::UNAUTHORIZED,
                    code: 3,
                    message: Some(message.to_string()),
                },
            }
        }

        pub fn internal(message: Option<String>) -> Self {
            Self {
                status: StatusCode::INTERNAL_SERVER_ERROR,
                code: 1, // placeholder
                message,
            }
        }
    }

    impl From<UseCaseError> for HttpError {
        fn from(err: UseCaseError) -> Self {
            match err.as_ref() {
                DomainError::Expected { variant, message } => Self::expected(variant, message),
                DomainError::Unrecoverable { .. } => {
                    tracing::warn!("[HttpError::from<UseCaseErr>] Unrecoverable error concealed");
                    Self::internal(Some(trl("error-internal")))
                }
            }
        }
    }

    impl From<JsonRejection> for HttpError {
        fn from(err: JsonRejection) -> Self {
            Self {
                status: StatusCode::BAD_REQUEST,
                code: 2,
                message: Some(err.body_text()),
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
        /// Only for Into<Response> implementation, not serialized in response body.
        #[serde(skip)]
        status: StatusCode,

        #[serde(skip)]
        headers: HeaderMap,

        /// Buisness-level status code. Always 0 for successful response, non-zero for error response.
        code: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
        #[serde(skip_serializing_if = "Option::is_none")]
        data: Option<T>,
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
                self.headers.insert(axum::http::header::SET_COOKIE, value);
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

    pub fn accept<T>(data: T) -> HttpResult<T>
    where
        T: Serialize,
    {
        Ok(HttpResponse::from(data))
    }

    pub trait Accept {
        type Data: Serialize;

        fn accept(self) -> HttpResult<Self::Data>;
    }

    impl<T> Accept for T
    where
        T: Serialize,
    {
        type Data = T;

        fn accept(self) -> HttpResult<Self::Data> {
            accept(self)
        }
    }
}
