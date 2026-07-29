use poprako_orchestra::Oper;

use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::read::spec::termbase::TermbaseListSpec;
use crate::model::write::termbase::{TermbaseEntry, TermbaseRepl};

/// Creates a termbase.
#[derive(Oper)]
#[oper(output = TermbaseInfo)]
pub struct CreateTermbase<'a> {
    /// The termbase entry to insert.
    pub entry: &'a TermbaseEntry,
}

/// Looks up a termbase by identifier.
#[derive(Oper)]
#[oper(output = TermbaseInfo)]
pub struct GetTermbaseInfo<'a> {
    /// The termbase id.
    pub id: &'a str,
}

/// Lists termbase infos matching a filter spec.
#[derive(Oper)]
#[oper(output = Vec<TermbaseInfo>)]
pub struct ListTermbaseInfos<'a> {
    /// The specification for filtering listed termbases.
    pub spec: &'a TermbaseListSpec,
}

/// Looks up a termbase by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = TermbaseInfo)]
pub struct GetTermbaseInfoExcluded<'a> {
    /// The termbase id.
    pub id: &'a str,
}

/// Lists termbase infos by owner, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = Vec<TermbaseInfo>)]
pub enum ListTermbaseInfosExcluded<'a> {
    //
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

/// Updates a termbase.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateTermbase<'a> {
    /// The update payload for the termbase.
    pub update: &'a TermbaseRepl,
}

/// Updates a termbase's cached term count.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateTermbaseTermCount<'a> {
    //
    /// The termbase id.
    pub id: &'a str,

    /// The delta to apply to the term count.
    pub delta: i32,
}

/// Touches a termbase to update its timestamp.
#[derive(Oper)]
#[oper(output = ())]
pub struct TouchTermbase<'a> {
    /// The termbase id.
    pub id: &'a str,
}

/// Deletes a termbase.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteTermbase<'a> {
    /// The termbase id.
    pub id: &'a str,
}
