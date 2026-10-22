//! Domain models for team announcements.

/// The data needed to insert a team announcement row.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementEntry<'a> {
    /// Unique identifier to insert for the new announcement row.
    pub id: String,

    /// Foreign key identifying the target team for this announcement.
    pub team_id: String,
    /// Foreign key identifying the posting user.
    pub user_id: String,

    /// Headline text for the new announcement.
    pub title: &'a str,
    /// Full body text for the new announcement.
    pub content: &'a str,
}

/// Mutable announcement content fields.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementRepl<'a> {
    /// Identifier of the announcement to update.
    pub id: String,

    /// Updated headline text.
    pub title: &'a str,
    /// Updated body content.
    pub content: &'a str,
}
