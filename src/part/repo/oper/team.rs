use poprako_orchestra::Oper;

use crate::model::team::{
    TeamAvatarReservation, TeamEntry, TeamInfo, TeamInfoListSpec,
};
use crate::value::image::{ImageExt, ImageHash};

/// Creates a team.
pub struct CreateTeam<'a> {
    /// The team entry to insert.
    pub entry: &'a TeamEntry,
}

impl Oper for CreateTeam<'_> {
    // Operation output type.
    type Output = TeamInfo;
}

/// Looks up a team by identifier.
pub enum GetTeamInfo<'a> {
    /// Fetch by team id.
    Id {
        /// The team identifier.
        id: &'a str,
    },
}

impl Oper for GetTeamInfo<'_> {
    // Operation output type.
    type Output = TeamInfo;
}

/// Lists team infos matching a filter spec.
pub struct ListTeamInfos<'a> {
    /// The specification for filtering listed teams.
    pub spec: &'a TeamInfoListSpec,
}

impl Oper for ListTeamInfos<'_> {
    // Operation output type.
    type Output = Vec<TeamInfo>;
}

/// Updates a team.
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

impl Oper for UpdateTeam<'_> {
    // Operation output type.
    type Output = ();
}

/// Reserves a team avatar slot for an upload.
pub struct ReserveTeamAvatar<'a> {
    //
    /// The team id.
    pub id: &'a str,

    /// The image hash for deduplication.
    pub image_hash: &'a ImageHash,

    /// The image file extension.
    pub image_ext: ImageExt,
}

impl Oper for ReserveTeamAvatar<'_> {
    // Operation output type.
    type Output = TeamAvatarReservation;
}

/// Looks up a team by identifier, matching deleted rows as well.
pub enum GetTeamInfoExcluded<'a> {
    /// Fetch by team id.
    Id {
        /// The team identifier.
        id: &'a str,
    },
}

impl Oper for GetTeamInfoExcluded<'_> {
    // Operation output type.
    type Output = TeamInfo;
}

/// Locks a team row.
pub struct LockTeam<'a> {
    /// The team id.
    pub id: &'a str,
}

impl Oper for LockTeam<'_> {
    // Operation output type.
    type Output = ();
}

/// Deletes a team.
pub struct DeleteTeam<'a> {
    /// The team id.
    pub id: &'a str,
}

impl Oper for DeleteTeam<'_> {
    // Operation output type.
    type Output = ();
}

/// Allocates a sequential index for a new workset under this team.
pub struct AllocTeamWorksetIndex<'a> {
    /// The team id.
    pub id: &'a str,
}

impl Oper for AllocTeamWorksetIndex<'_> {
    // Operation output type.
    type Output = i32;
}
