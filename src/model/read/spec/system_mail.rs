//! Domain models for system mail notifications.

/// Filtering and pagination parameters for listing a user's system mail.
pub struct SystemMailListSpec {
    //
    /// The user whose mail inbox is being queried.
    pub receiver_id: String,
    /// Read-status filter mode for the mail listing.
    pub kind: SystemMailListKind,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}

/// Read-status filtering mode for listing system mail.
pub enum SystemMailListKind {
    //
    /// Include mail regardless of read status.
    All,

    /// Include only read mail.
    Read,

    /// Include only unread mail.
    Unread,
}
