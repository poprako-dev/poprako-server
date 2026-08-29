//! Domain models for team- and comic-scoped terminology bases.

use crate::model::write::term::TermImport;

/// The data needed to create a terminology base.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseEntry {
    /// The unique identifier for the new terminology base.
    pub id: String,

    /// Foreign key of the owning team, if creating a team-scoped termbase.
    pub team_id: Option<String>,
    /// Foreign key of the associated comic, if creating a comic-scoped termbase.
    pub comic_id: Option<String>,

    /// Display name for the new terminology base.
    pub name: String,
    /// Free-text description for the new terminology base.
    pub description: Option<String>,

    /// Foreign key of the user creating this terminology base.
    pub creator_id: String,
}

/// Portable terminology-base content supplied for import.
pub struct TermbaseImport {
    /// Display name for the imported terminology base.
    pub name: String,
    /// Optional description for the imported terminology base.
    pub description: Option<String>,
    /// Portable terminology entries included in the document.
    pub terms: Vec<TermImport>,
}

/// Mutable terminology-base profile fields.
#[cfg_attr(test, derive(Clone))]
pub struct TermbaseRepl {
    /// The unique identifier of the termbase to update.
    pub id: String,

    /// Updated display name for the terminology base.
    pub name: String,
    /// Updated description for the terminology base.
    pub description: Option<String>,
}
