//! Domain models for system mail notifications.

use time::OffsetDateTime;

/// A system mail record as stored in the database.
///
/// Carries a raw [`OffsetDateTime`] timestamp; convert to [`SystemMailVal`] for
/// presentation.
///
/// [`SystemMailInfoVal`]: crate::data::system_mail::SystemMailInfoVal
#[cfg_attr(test, derive(Clone))]
pub struct SystemMailInfo {
    //
    /// The unique identifier for this system mail record.
    pub id: String,

    /// Foreign key to the user who received this notification.
    pub receiver_id: String,

    /// Whether the recipient has marked this mail as read.
    pub is_read: bool,

    /// Subject line of the system mail notification.
    pub title: String,
    /// Body text of the system mail notification.
    pub content: String,

    /// Timestamp when this system mail was sent.
    pub created_at: OffsetDateTime,
}

/// The data needed to insert a new system mail row.
#[cfg_attr(test, derive(Clone))]
pub struct SystemMailEntry {
    //
    /// The unique identifier for the new system mail record.
    pub id: String,

    /// Foreign key of the user who should receive this mail.
    pub receiver_id: String,

    /// Subject line of the system mail to send.
    pub title: String,
    /// Body text of the system mail to send.
    pub content: String,
}

/// Filtering and pagination parameters for listing a user's system mail.
pub struct SystemMailInfoListSpec {
    //
    /// The user whose mail inbox is being queried.
    pub receiver_id: String,
    /// Read-status filter mode for the mail listing.
    pub kind: SystemMailInfoListKind,

    /// Number of records to skip for pagination.
    pub offset: u32,
    /// Maximum number of records to return in this page.
    pub limit: u32,
}

/// Read-status filtering mode for listing system mail.
pub enum SystemMailInfoListKind {
    /// Include mail regardless of read status.
    All,

    /// Include only read mail.
    Read,

    /// Include only unread mail.
    Unread,
}
