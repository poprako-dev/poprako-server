//! View DTOs for the system-mail domain.
/// Presentation-ready system mail value.
///
/// Converts the raw [`SystemMailInfo`] timestamp to Unix milliseconds
/// and omits the internal `receiver_id` field.
pub struct SystemMailInfoView {
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
