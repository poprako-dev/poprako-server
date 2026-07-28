//! Val DTOs for the system mail domain.

//! Data transfer objects for system mail use cases.

use serde::Serialize;

#[cfg(feature = "swagger")]
use utoipa::ToSchema;

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
    pub is_read: bool,

    /// Timestamp of creation, in Unix milliseconds.
    pub created_at: i64,
}
