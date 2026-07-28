use poprako_orchestra::Oper;

use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListSpec,
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

/// Lists team infos matching a filter spec.
#[derive(Oper)]
#[oper(output = Vec<TeamInfo>)]
pub struct ListTeamInfos<'a> {
    /// The specification for filtering listed teams.
    pub spec: &'a TeamInfoListSpec,
}

/// Updates a team.
#[derive(Oper)]
#[oper(output = ())]
pub enum UpdateTeam<'a> {
    /// Updates team metadata fields.
    Info {
        //
        /// The team identifier.
        id: &'a str,
        /// The display name.
        name: &'a str,
        /// A short description of the team.
        description: &'a str,
    },

    /// Marks a team avatar as uploaded.
    MarkAvatarUploaded {
        //
        /// The team identifier.
        id: &'a str,
        /// The new avatar version number.
        avatar_version: u32,
        /// The object storage key.
        avatar_key: Option<&'a str>,
        /// Whether the upload has completed.
        avatar_uploaded: bool,
    },
}

/// Reserves a team avatar slot for an upload.
#[derive(Oper)]
#[oper(output = TeamAvatarReservation)]
pub struct ReserveTeamAvatar<'a> {
    //
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
