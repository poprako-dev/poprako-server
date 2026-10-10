//! Stable composite artwork page identities.

use time::OffsetDateTime;

/// One independently ordered composite page belonging to a Chapter.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PageArtworkInfo {
    /// Stable page identity.
    pub id: String,

    /// Owning Chapter.
    pub chapter_id: String,
    /// Zero-based composite page position.
    pub index: usize,

    /// Optional complete original PSD filename or relative path.
    pub raw_ident: Option<String>,

    /// Creation time.
    pub created_at: OffsetDateTime,
    /// Last composite page metadata update.
    pub updated_at: OffsetDateTime,
}
