//! Domain models for team profile storage.

use std::borrow::Cow;

/// The data needed to create a new team.
#[cfg_attr(test, derive(Clone))]
pub struct TeamEntry<'a> {
    /// The unique identifier for the new team.
    pub id: String,

    /// Display name for the team being created.
    pub name: Cow<'a, str>,
    /// Free-text description for the team being created.
    pub description: Cow<'a, str>,
}

/// Mutable team profile fields updated together.
pub struct TeamRepl<'a> {
    /// The team identifier.
    pub id: String,

    /// The display name.
    pub name: &'a str,
    /// The team description.
    pub description: &'a str,
}
