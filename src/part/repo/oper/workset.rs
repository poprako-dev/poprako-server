use poprako_orchestra::Oper;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::{WorksetEntry, WorksetRepl};

/// Creates a workset.
#[derive(Oper)]
#[oper(output = WorksetInfo)]
pub struct CreateWorkset<'a> {
    /// The workset entry to insert.
    pub entry: &'a WorksetEntry,
}

/// Looks up a workset by identifier.
#[derive(Oper)]
#[oper(output = WorksetInfo)]
pub struct GetWorksetInfo<'a> {
    /// The workset id.
    pub id: &'a str,
}

/// Lists workset infos for a team with pagination.
#[derive(Oper)]
#[oper(output = Vec<WorksetInfo>)]
pub struct ListWorksetInfos<'a> {
    /// The team id.
    pub team_id: &'a str,

    /// The pagination offset.
    pub offset: u32,

    /// The pagination limit.
    pub limit: u32,
}

/// Looks up a workset by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = WorksetInfo)]
pub struct GetWorksetInfoExcluded<'a> {
    /// The workset id.
    pub id: &'a str,
}

/// Lists workset infos for a team, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = Vec<WorksetInfo>)]
pub struct ListWorksetInfosExcluded<'a> {
    /// The team id.
    pub team_id: &'a str,
}

/// Updates a workset.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateWorkset<'a> {
    /// The update payload for the workset.
    pub update: &'a WorksetRepl,
}

/// Deletes a workset.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteWorkset<'a> {
    /// The workset id.
    pub id: &'a str,
}

/// Allocates a sequential index for a new comic under this workset.
#[derive(Oper)]
#[oper(output = usize)]
pub struct AllocWorksetComicIndex<'a> {
    /// The workset id.
    pub id: &'a str,
}

/// Updates a workset's cached comic count.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateWorksetComicCount<'a> {
    /// The workset id.
    pub id: &'a str,

    /// The delta to apply to the comic count.
    pub delta: i32,
}
