//! Data transfer objects for system mail use cases.

use poprako_util::page::Page;
use poprako_util::time::ToUnixMilli;

use crate::model::system_mail::SystemMailInfo;

/// Input parameters for listing system mails.
pub struct ListSystemMailData {
    pub read: Option<bool>,
    pub page: Page,
}

/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
pub struct SystemMailVal {
    pub id: String,
    pub title: String,
    pub content: String,
    pub read: bool,
    pub created_at: i64,
}

impl SystemMailVal {
    /// Converts a [`SystemMailInfo`] into a presentation-ready value.
    ///
    /// The `created_at` timestamp is converted from [`OffsetDateTime`] to
    /// Unix milliseconds.
    ///
    /// [`OffsetDateTime`]: time::OffsetDateTime
    pub fn from_model(model: SystemMailInfo) -> Self {
        Self {
            id: model.id,
            title: model.title,
            content: model.content,
            read: model.read,
            created_at: model.created_at.to_unix_milli(),
        }
    }
}
