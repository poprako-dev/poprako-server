//! Data transfer objects for authentication use cases.

use serde::{Deserialize, Serialize};

use utoipa::ToSchema;

/// Input parameters for user registration.
#[derive(Debug, Deserialize, ToSchema)]
pub struct RegisterData {
    pub qid: String,
    pub nickname: String,

    pub password: String,

    pub code: String,
}

/// Return value from a successful registration.
#[derive(Debug, Serialize, ToSchema)]
pub struct RegisterVal {
    pub user_id: String,
    pub token: String,
}

/// Input parameters for user login.
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoginData {
    pub qid: String,
    pub password: String,
}

/// Return value from a successful login.
#[derive(Debug, Serialize, ToSchema)]
pub struct LoginVal {
    pub user_id: String,
    pub token: String,
}
