use poprako_orchestra::Oper;

use crate::model::termbase::{
    TermbaseEntry, TermbaseInfo, TermbaseInfoListSpec, TermbaseInfoUpdate,
};

/// Creates a termbase.
pub struct CreateTermbase<'a> {
    /// The termbase entry to insert.
    pub entry: &'a TermbaseEntry,
}

impl Oper for CreateTermbase<'_> {
    // Operation output type.
    type Output = TermbaseInfo;
}

/// Looks up a termbase by identifier.
pub struct GetTermbaseInfo<'a> {
    /// The termbase id.
    pub id: &'a str,
}

impl Oper for GetTermbaseInfo<'_> {
    // Operation output type.
    type Output = TermbaseInfo;
}

/// Lists termbase infos matching a filter spec.
pub struct ListTermbaseInfos<'a> {
    /// The specification for filtering listed termbases.
    pub spec: &'a TermbaseInfoListSpec,
}

impl Oper for ListTermbaseInfos<'_> {
    // Operation output type.
    type Output = Vec<TermbaseInfo>;
}

/// Looks up a termbase by identifier, matching deleted rows as well.
pub struct GetTermbaseInfoExcluded<'a> {
    /// The termbase id.
    pub id: &'a str,
}

impl Oper for GetTermbaseInfoExcluded<'_> {
    // Operation output type.
    type Output = TermbaseInfo;
}

/// Lists termbase infos by owner, matching deleted rows as well.
pub enum ListTermbaseInfosExcluded<'a> {
    /// Fetch all termbases for a team.
    Team {
        /// The team identifier.
        team_id: &'a str,
    },

    /// Fetch all termbases for a comic.
    Comic {
        /// The comic identifier.
        comic_id: &'a str,
    },
}

impl Oper for ListTermbaseInfosExcluded<'_> {
    // Operation output type.
    type Output = Vec<TermbaseInfo>;
}

/// Updates a termbase.
pub struct UpdateTermbase<'a> {
    /// The update payload for the termbase.
    pub update: &'a TermbaseInfoUpdate,
}

impl Oper for UpdateTermbase<'_> {
    // Operation output type.
    type Output = ();
}

/// Updates a termbase's cached term count.
pub struct UpdateTermbaseTermCount<'a> {
    //
    /// The termbase id.
    pub id: &'a str,

    /// The delta to apply to the term count.
    pub delta: i32,
}

impl Oper for UpdateTermbaseTermCount<'_> {
    // Operation output type.
    type Output = ();
}

/// Touches a termbase to update its timestamp.
pub struct TouchTermbase<'a> {
    /// The termbase id.
    pub id: &'a str,
}

impl Oper for TouchTermbase<'_> {
    // Operation output type.
    type Output = ();
}

/// Deletes a termbase.
pub struct DeleteTermbase<'a> {
    /// The termbase id.
    pub id: &'a str,
}

impl Oper for DeleteTermbase<'_> {
    // Operation output type.
    type Output = ();
}
