use poprako_orchestra::Oper;

use crate::model::termbase::{
    TermbaseEntry, TermbaseInfo, TermbaseInfoListSpec, TermbaseInfoUpdate,
};

pub struct CreateTermbase<'a> {
    pub entry: &'a TermbaseEntry,
}

impl<'a> Oper for CreateTermbase<'a> {
    type Output = TermbaseInfo;
}

pub struct GetTermbaseInfo<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetTermbaseInfo<'a> {
    type Output = TermbaseInfo;
}

pub struct ListTermbaseInfos<'a> {
    pub spec: &'a TermbaseInfoListSpec,
}

impl<'a> Oper for ListTermbaseInfos<'a> {
    type Output = Vec<TermbaseInfo>;
}

pub struct GetTermbaseInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetTermbaseInfoExcluded<'a> {
    type Output = TermbaseInfo;
}

pub enum ListTermbaseInfosExcluded<'a> {
    Team { team_id: &'a str },
    Comic { comic_id: &'a str },
}

impl<'a> Oper for ListTermbaseInfosExcluded<'a> {
    type Output = Vec<TermbaseInfo>;
}

pub struct UpdateTermbase<'a> {
    pub update: &'a TermbaseInfoUpdate,
}

impl<'a> Oper for UpdateTermbase<'a> {
    type Output = ();
}

pub struct UpdateTermbaseTermCount<'a> {
    pub id: &'a str,
    pub delta: i32,
}

impl<'a> Oper for UpdateTermbaseTermCount<'a> {
    type Output = ();
}

pub struct TouchTermbase<'a> {
    pub id: &'a str,
}

impl<'a> Oper for TouchTermbase<'a> {
    type Output = ();
}

pub struct DeleteTermbase<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteTermbase<'a> {
    type Output = ();
}
