use poprako_orchestra::Oper;

use crate::model::workset::{WorksetEntry, WorksetInfo, WorksetInfoUpdate};

pub struct CreateWorkset<'a> {
    pub entry: &'a WorksetEntry,
}

impl Oper for CreateWorkset<'_> {
    type Output = WorksetInfo;
}

pub struct GetWorksetInfo<'a> {
    pub id: &'a str,
}

impl Oper for GetWorksetInfo<'_> {
    type Output = WorksetInfo;
}

pub struct ListWorksetInfos<'a> {
    pub team_id: &'a str,
    pub offset: u32,
    pub limit: u32,
}

impl Oper for ListWorksetInfos<'_> {
    type Output = Vec<WorksetInfo>;
}

pub struct GetWorksetInfoExcluded<'a> {
    pub id: &'a str,
}

impl Oper for GetWorksetInfoExcluded<'_> {
    type Output = WorksetInfo;
}

pub struct ListWorksetInfosExcluded<'a> {
    pub team_id: &'a str,
}

impl Oper for ListWorksetInfosExcluded<'_> {
    type Output = Vec<WorksetInfo>;
}

pub struct UpdateWorkset<'a> {
    pub update: &'a WorksetInfoUpdate,
}

impl Oper for UpdateWorkset<'_> {
    type Output = ();
}

pub struct DeleteWorkset<'a> {
    pub id: &'a str,
}

impl Oper for DeleteWorkset<'_> {
    type Output = ();
}

pub struct AllocWorksetComicIndex<'a> {
    pub id: &'a str,
}

impl Oper for AllocWorksetComicIndex<'_> {
    type Output = i32;
}

pub struct UpdateWorksetComicCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl Oper for UpdateWorksetComicCount<'_> {
    type Output = ();
}
