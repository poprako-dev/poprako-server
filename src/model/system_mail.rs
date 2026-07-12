//! Domain models for system mail notifications.

use time::OffsetDateTime;

/// A system mail record as stored in the database.
///
/// Carries a raw [`OffsetDateTime`] timestamp; convert to [`SystemMailVal`] for
/// presentation.
///
/// [`SystemMailVal`]: crate::data::system_mail::SystemMailVal
#[cfg_attr(test, derive(Clone))]
pub struct Info {
    pub id: String,

    pub receiver_id: String,

    pub read: bool,

    pub title: String,
    pub content: String,

    pub created_at: OffsetDateTime,
}

/// The data needed to insert a new system mail row.
#[cfg_attr(test, derive(Clone))]
pub struct Form {
    pub id: String,

    pub receiver_id: String,

    pub title: String,
    pub content: String,
}
