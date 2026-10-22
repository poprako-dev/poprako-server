//! Domain models for worksets — units of work within a team.

/// The data needed to create a new workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetEntry<'a> {
    /// Server-assigned unique workset identifier.
    pub id: String,

    /// Foreign key referencing the owning team.
    pub team_id: String,
    /// Display ordering index within the team.
    pub index: usize,

    /// Human-readable workset name.
    pub name: &'a str,
    /// Optional longer description of this workset's purpose.
    pub description: Option<&'a str>,
}

/// Mutable profile fields for a workset.
#[cfg_attr(test, derive(Clone))]
pub struct WorksetRepl<'a> {
    /// Server-assigned identifier of the workset to update.
    pub id: String,
    /// Updated human-readable workset name.
    pub name: &'a str,
    /// Updated description; `None` means no description.
    pub description: Option<&'a str>,
}
