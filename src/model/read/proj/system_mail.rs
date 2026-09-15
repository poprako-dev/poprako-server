//! Domain models for system mail notifications.

use time::OffsetDateTime;

/// A system mail record as stored in the database.
///
/// Carries a raw [`OffsetDateTime`] timestamp; convert to [`SystemMailVal`] for
/// presentation.
///
/// [`SystemMailInfoView`]: crate::data::view::system_mail::SystemMailInfoView
#[cfg_attr(test, derive(Clone))]
pub struct SystemMailInfo {
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
