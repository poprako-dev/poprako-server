//! Data transfer objects for system mail use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger-ui")]
use utoipa::{IntoParams, ToSchema};

/// Input parameters for listing system mails.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(IntoParams))]
#[cfg_attr(feature = "swagger-ui", into_params(parameter_in = Query))]
pub struct ListData {
    pub read: Option<bool>,

    pub offset: u32,
    pub limit: u32,
}

/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct Val {
    pub id: String,

    pub title: String,
    pub content: String,

    pub read: bool,

    pub created_at: i64,
}

/// Input parameters for marking a batch of system mails as read.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub struct MarkReadData {
    pub ids: Vec<String>,
}
