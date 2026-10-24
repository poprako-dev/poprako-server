//! Domain models for team announcements.

/// The data needed to insert a team announcement row.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementEntry {
    /// Unique identifier to insert for the new announcement row.
    pub id: String,

    /// Foreign key identifying the target team for this announcement.
    pub team_id: String,
    /// Foreign key identifying the posting user.
    pub user_id: String,

    /// Headline text for the new announcement.
    pub title: String,
    /// Full body text for the new announcement.
    pub content: String,
}

impl AnnouncementEntry {
    /// Borrows normalized title without copying its storage.
    pub fn trimmed_title(&self) -> &str {
        self.title.trim()
    }

    /// Borrows normalized content without copying its storage.
    pub fn trimmed_content(&self) -> &str {
        self.content.trim()
    }
}

/// Mutable announcement content fields.
#[cfg_attr(test, derive(Clone))]
pub struct AnnouncementRepl {
    /// Identifier of the announcement to update.
    pub id: String,

    /// Updated headline text.
    pub title: String,
    /// Updated body content.
    pub content: String,
}

impl AnnouncementRepl {
    /// Borrows normalized title without copying its storage.
    pub fn trimmed_title(&self) -> &str {
        self.title.trim()
    }

    /// Borrows normalized content without copying its storage.
    pub fn trimmed_content(&self) -> &str {
        self.content.trim()
    }
}
