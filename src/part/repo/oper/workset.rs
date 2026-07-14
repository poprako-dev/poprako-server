use poprako_orchestra::Oper;

use poprako_util::page::Page;

use crate::model::workset::{WorksetEntry, WorksetInfo, WorksetInfoUpdate};

pub struct CreateWorkset<'a> {
    pub entry: &'a WorksetEntry,
}

impl<'a> Oper for CreateWorkset<'a> {
    type Output = WorksetInfo;
}

pub struct GetWorksetInfo<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetWorksetInfo<'a> {
    type Output = WorksetInfo;
}

pub struct ListWorksetInfos<'a> {
    pub team_id: &'a str,
    pub page: Option<Page>,
}

impl<'a> Oper for ListWorksetInfos<'a> {
    type Output = Vec<WorksetInfo>;
}

pub struct GetWorksetInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetWorksetInfoExcluded<'a> {
    type Output = WorksetInfo;
}

pub struct ListWorksetInfosExcluded<'a> {
    pub team_id: &'a str,
}

impl<'a> Oper for ListWorksetInfosExcluded<'a> {
    type Output = Vec<WorksetInfo>;
}

pub struct UpdateWorkset<'a> {
    pub update: &'a WorksetInfoUpdate,
}

impl<'a> Oper for UpdateWorkset<'a> {
    type Output = ();
}

pub struct DeleteWorkset<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteWorkset<'a> {
    type Output = ();
}

pub struct AllocWorksetComicIndex<'a> {
    pub id: &'a str,
}

impl<'a> Oper for AllocWorksetComicIndex<'a> {
    type Output = i32;
}

pub struct UpdateWorksetComicCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl<'a> Oper for UpdateWorksetComicCount<'a> {
    type Output = ();
}
