//! Data transfer objects for system mail use cases.

use poprako_macro::Paginate;

/// Input parameters for listing system mails.
#[Paginate]
pub struct ListSystemMailData {
    pub read: Option<bool>,
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
