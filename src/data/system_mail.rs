//! Data transfer objects for system mail use cases.

use serde::{Deserialize, Serialize};

#[cfg(feature = "swagger")]
use utoipa::{IntoParams, ToSchema};

/// Input parameters for listing system mails.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(IntoParams))]
#[cfg_attr(feature = "swagger", into_params(parameter_in = Query))]
pub struct ListSystemMailInfosParams {
    //
    /// Filter by read status. Absent returns all.
    pub read: Option<bool>,

    /// Pagination offset.
    pub offset: u32,
    /// Maximum number of results per page.
    pub limit: u32,
}

/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct SystemMailInfoVal {
    //
    /// Unique identifier.
    pub id: String,

    /// Mail title.
    pub title: String,
    /// Mail body content.
    pub content: String,

    /// Whether the mail has been read.
    pub read: bool,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
}

/// Input parameters for marking a batch of system mails as read.
#[derive(Debug, Deserialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MarkSystemMailReadParams {
    /// Identifiers of the system mails to mark as read.
    pub ids: Vec<String>,
}
