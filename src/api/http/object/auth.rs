//! HTTP wire objects for the migrated data DTOs.
use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

#[cfg(not(debug_assertions))]
use veil::Redact;

use crate::api::http::result::Wire;
use crate::data::instr::auth::{LoginAuthInstr, RegisterAuthInstr};
use crate::data::val::auth::{LoginAuthVal, RegisterAuthVal};

/// Input parameters for user registration.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthBody {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// Display name for the user.
    pub nickname: String,

    /// User password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub password: String,

    /// Verification code.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub code: String,
}

impl From<RegisterAuthBody> for RegisterAuthInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: RegisterAuthBody) -> Self {
        Self {
            qid: value.qid,
            nickname: value.nickname,
            password: value.password,
            code: value.code,
        }
    }
}

/// Input parameters for user login.
#[derive(Deserialize)]
#[cfg_attr(debug_assertions, derive(Debug))]
#[cfg_attr(not(debug_assertions), derive(Redact))]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthBody {
    /// Unique user identifier for authentication.
    pub qid: String,
    /// User password.
    #[cfg_attr(not(debug_assertions), redact(fixed = 10, with = "[REDACTED]"))]
    pub password: String,
}

impl From<LoginAuthBody> for LoginAuthInstr {
    // Converts the value between the use-case and wire shapes.
    fn from(value: LoginAuthBody) -> Self {
        Self {
            qid: value.qid,
            password: value.password,
        }
    }
}

/// Return value from a successful registration.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct RegisterAuthPayload {
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}

impl From<RegisterAuthVal> for RegisterAuthPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: RegisterAuthVal) -> Self {
        Self {
            user_id: value.user_id,
            token: value.token,
        }
    }
}

impl Wire for RegisterAuthVal {
    // Serializable wire payload produced by this conversion.
    type Target = RegisterAuthPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}

/// Return value from a successful login.
#[derive(Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct LoginAuthPayload {
    /// Unique user identifier.
    pub user_id: String,
    /// Authentication token.
    pub token: String,
}

impl From<LoginAuthVal> for LoginAuthPayload {
    // Converts the value between the use-case and wire shapes.
    fn from(value: LoginAuthVal) -> Self {
        Self {
            user_id: value.user_id,
            token: value.token,
        }
    }
}

impl Wire for LoginAuthVal {
    // Serializable wire payload produced by this conversion.
    type Target = LoginAuthPayload;

    // Converts the use-case value into its wire representation.
    fn into_wire(self) -> Self::Target {
        self.into()
    }
}
