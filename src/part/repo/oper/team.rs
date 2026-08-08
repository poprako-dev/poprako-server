use poprako_orchestra::Oper;

use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{
    TeamAvatarRepl, TeamAvatarReservation, TeamEntry, TeamRepl,
};
use crate::value::image::{ImageExt, ImageHash};

/// Creates a team.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub struct CreateTeam<'a> {
    /// The team entry to insert.
    pub entry: &'a TeamEntry,
}

/// Looks up a team by identifier.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub enum GetTeamInfo<'a> {
    /// Fetch by team id.
    Id {
        /// The team identifier.
        id: &'a str,
    },
}

/// Resolves the owning team from a nested domain resource.
#[derive(Oper)]
#[oper(output = String)]
pub enum ResolveTeamId<'a> {
    //
    /// Resolve from a comic identifier.
    Comic {
        /// The comic identifier.
        id: &'a str,
    },

    /// Resolve from a chapter identifier.
    Chapter {
        /// The chapter identifier.
        id: &'a str,
    },
}

/// Lists team infos matching a filter spec.
#[derive(Oper)]
#[oper(output = Vec<TeamInfo>)]
pub struct ListTeamInfos<'a> {
    /// The specification for filtering listed teams.
    pub spec: &'a TeamListSpec,
}

/// Updates a team.
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateTeam<'a> {
    //
    /// Updates team metadata fields.
    Info {
        /// The replacement payload.
        repl: &'a TeamRepl,
    },

    /// Marks a team avatar as uploaded.
    MarkAvatarUploaded {
        /// The replacement payload.
        repl: &'a TeamAvatarRepl,
    },
}

/// Reserves a team avatar slot for an upload.
#[derive(Oper)]
#[oper(output = TeamAvatarReservation)]
pub struct ReserveTeamAvatar<'a> {
    /// The team id.
    pub id: &'a str,

    /// The image hash for deduplication.
    pub image_hash: &'a ImageHash,

    /// The image file extension.
    pub image_ext: ImageExt,
}

/// Looks up a team by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = TeamInfo)]
pub enum GetTeamInfoExcluded<'a> {
    /// Fetch by team id.
    Id {
        /// The team identifier.
        id: &'a str,
    },
}

/// Locks a team row.
#[derive(Oper)]
#[oper(output = ())]
pub struct LockTeam<'a> {
    /// The team id.
    pub id: &'a str,
}

/// Deletes a team.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteTeam<'a> {
    /// The team id.
    pub id: &'a str,
}

/// Allocates a sequential index for a new workset under this team.
#[derive(Oper)]
#[oper(output = i32)]
pub struct AllocTeamWorksetIndex<'a> {
    /// The team id.
    pub id: &'a str,
}
