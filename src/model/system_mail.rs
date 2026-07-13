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
    pub id: String,

    pub receiver_id: String,

    pub read: bool,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,
}

/// The data needed to insert a new system mail row.
#[cfg_attr(test, derive(Clone))]
pub struct SystemMailEntry {
    pub id: String,

    pub receiver_id: String,

    pub title: String,
    pub content: String,
}
