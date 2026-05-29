pub mod authorization;
pub mod user;

pub use result::{HttpError, HttpResult};

mod result {
    use axum::Json;
    use axum::extract::rejection::JsonRejection;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use serde::Serialize;
    use utoipa::ToSchema;

    use crate::domain::result::{DomainError, ExpectedVariant};
    use crate::usecase::result::{UseCaseError, UseCaseResult};
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
                code: 0,
                message: None,
                data: Some(data),
            }
        }
    }

    impl<T> IntoResponse for HttpResponse<T>
    where
        T: Serialize,
    {
        fn into_response(self) -> Response {
            (self.status, Json(self)).into_response()
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
