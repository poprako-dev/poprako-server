pub mod authorization;
pub mod user;

pub use result::{HttpError, HttpResl};

mod result {
    use axum::Json;
    use axum::extract::rejection::JsonRejection;
    use axum::http::StatusCode;
    use axum::response::{IntoResponse, Response};
    use serde::Serialize;
    use utoipa::ToSchema;

    use crate::domain::result::{DomainErr, ExpectedErr};
    use crate::usecase::result::UseCaseErr;
    use crate::util::i18n::trl;
    use crate::util::rename::StdResl;

    #[derive(Debug, Serialize, ToSchema)]
    pub struct HttpError {
        #[serde(skip)]
        status: StatusCode,

        code: u16,
        #[serde(skip_serializing_if = "Option::is_none")]
        message: Option<String>,
    }

    impl HttpError {
        pub fn expected(variant: &ExpectedErr, message: &str) -> Self {
            match variant {
                ExpectedErr::Argument => Self {
                    status: StatusCode::BAD_REQUEST,
                    code: 2,
                    message: Some(message.to_string()),
                },
                ExpectedErr::Authentication => Self {
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

    impl From<UseCaseErr> for HttpError {
        fn from(err: UseCaseErr) -> Self {
            match err.as_ref() {
                DomainErr::Expected { variant, message } => Self::expected(variant, message),
                DomainErr::Unrecoverable { .. } => {
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

    pub type HttpResl<T> = StdResl<HttpResponse<T>, HttpError>;

    pub fn accept<T>(data: T) -> HttpResl<T>
    where
        T: Serialize,
    {
        Ok(HttpResponse::from(data))
    }
}
