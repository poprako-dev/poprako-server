//! Data transfer objects for authentication use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

/// Input parameters for user registration.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct RegisterAuthParams {
    pub qid: String,
    pub nickname: String,

    pub password: String,

    pub code: String,
}

/// Return value from a successful registration.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct RegisterAuthPayload {
    pub user_id: String,
    pub token: String,
}

/// Input parameters for user login.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct LoginAuthParams {
    pub qid: String,
    pub password: String,
}

/// Return value from a successful login.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct LoginAuthPayload {
    pub user_id: String,
    pub token: String,
}
