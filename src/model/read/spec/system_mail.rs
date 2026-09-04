//! Domain models for system mail notifications.

use crate::value::pagination::PubListLimit;

/// Filtering and pagination parameters for listing a user's system mail.
pub struct SystemMailListSpec {
    //
    /// The user whose mail inbox is being queried.
    pub receiver_id: String,
    /// Optional read-status filter.
    pub is_read: Option<bool>,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: PubListLimit,
}
