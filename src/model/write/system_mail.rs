//! Domain models for system mail notifications.

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
