//! Domain models for system mail notifications.

use poprako_macro::Paginate;
use time::OffsetDateTime;

/// A system mail record as stored in the database.
///
/// Carries a raw [`OffsetDateTime`] timestamp; convert to [`SystemMailVal`] for
/// presentation.
///
/// [`SystemMailVal`]: crate::data::system_mail::SystemMailVal
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
pub struct SystemMailForm {
    pub id: String,
    pub receiver_id: String,
    pub title: String,
    pub content: String,
}

/// Filtering and pagination parameters for listing system mails.
///
/// When `read` is [`Some`] the repo filters to matching status;
/// [`None`] returns mails regardless of read status.
#[Paginate]
pub struct SystemMailListSpec {
    pub read: Option<bool>,
}
