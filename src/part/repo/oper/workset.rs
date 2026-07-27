use poprako_orchestra::Oper;

use crate::model::workset::{WorksetEntry, WorksetInfo, WorksetInfoUpdate};

/// Creates a workset.
pub struct CreateWorkset<'a> {
    /// The workset entry to insert.
    pub entry: &'a WorksetEntry,
}

impl Oper for CreateWorkset<'_> {
    // Operation output type.
    type Output = WorksetInfo;
}

/// Looks up a workset by identifier.
pub struct GetWorksetInfo<'a> {
    /// The workset id.
    pub id: &'a str,
}

impl Oper for GetWorksetInfo<'_> {
    // Operation output type.
    type Output = WorksetInfo;
}

/// Lists workset infos for a team with pagination.
pub struct ListWorksetInfos<'a> {
    //
    /// The team id.
    pub team_id: &'a str,

    /// The pagination offset.
    pub offset: u32,

    /// The pagination limit.
    pub limit: u32,
}

impl Oper for ListWorksetInfos<'_> {
    // Operation output type.
    type Output = Vec<WorksetInfo>;
}

/// Looks up a workset by identifier, matching deleted rows as well.
pub struct GetWorksetInfoExcluded<'a> {
    /// The workset id.
    pub id: &'a str,
}

impl Oper for GetWorksetInfoExcluded<'_> {
    // Operation output type.
    type Output = WorksetInfo;
}

/// Lists workset infos for a team, matching deleted rows as well.
pub struct ListWorksetInfosExcluded<'a> {
    /// The team id.
    pub team_id: &'a str,
}

impl Oper for ListWorksetInfosExcluded<'_> {
    // Operation output type.
    type Output = Vec<WorksetInfo>;
}

/// Updates a workset.
pub struct UpdateWorkset<'a> {
    /// The update payload for the workset.
    pub update: &'a WorksetInfoUpdate,
}

impl Oper for UpdateWorkset<'_> {
    // Operation output type.
    type Output = ();
}

/// Deletes a workset.
pub struct DeleteWorkset<'a> {
    /// The workset id.
    pub id: &'a str,
}

impl Oper for DeleteWorkset<'_> {
    // Operation output type.
    type Output = ();
}

/// Allocates a sequential index for a new comic under this workset.
pub struct AllocWorksetComicIndex<'a> {
    /// The workset id.
    pub id: &'a str,
}

impl Oper for AllocWorksetComicIndex<'_> {
    // Operation output type.
    type Output = i32;
}

/// Updates a workset's cached comic count.
pub struct UpdateWorksetComicCount<'a> {
    //
    /// The workset id.
    pub id: &'a str,

    /// The delta to apply to the comic count.
    pub delta: i32,
}

impl Oper for UpdateWorksetComicCount<'_> {
    // Operation output type.
    type Output = ();
}
