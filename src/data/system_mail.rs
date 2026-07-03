//! Data transfer objects for system mail use cases.

use serde::{Deserialize, Serialize};

use utoipa::{IntoParams, ToSchema};

use poprako_macro::Paginate;

/// Input parameters for listing system mails.
#[Paginate]
#[derive(Debug, Deserialize, IntoParams)]
#[into_params(parameter_in = Query)]
pub struct ListSystemMailData {
    pub read: Option<bool>,
}

/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
#[derive(Debug, Serialize, ToSchema)]
pub struct SystemMailVal {
    pub id: String,

    pub title: String,
    pub content: String,

    pub read: bool,

    pub created_at: i64,
}

/// Input parameters for marking a batch of system mails as read.
#[derive(Debug, Deserialize, ToSchema)]
pub struct MarkSystemMailsReadData {
    pub ids: Vec<String>,
}
