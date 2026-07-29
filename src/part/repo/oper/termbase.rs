use poprako_orchestra::Oper;

use crate::model::termbase::{
    TermbaseEntry, TermbaseInfo, TermbaseInfoListSpec, TermbaseInfoUpdate,
};

pub struct CreateTermbase<'a> {
    pub entry: &'a TermbaseEntry,
}

impl Oper for CreateTermbase<'_> {
    type Output = TermbaseInfo;
}

pub struct GetTermbaseInfo<'a> {
    pub id: &'a str,
}

impl Oper for GetTermbaseInfo<'_> {
    type Output = TermbaseInfo;
}

pub struct ListTermbaseInfos<'a> {
    pub spec: &'a TermbaseInfoListSpec,
}

impl Oper for ListTermbaseInfos<'_> {
    type Output = Vec<TermbaseInfo>;
}

pub struct GetTermbaseInfoExcluded<'a> {
    pub id: &'a str,
}

impl Oper for GetTermbaseInfoExcluded<'_> {
    type Output = TermbaseInfo;
}

pub enum ListTermbaseInfosExcluded<'a> {
    Team { team_id: &'a str },

    Comic { comic_id: &'a str },
}

impl Oper for ListTermbaseInfosExcluded<'_> {
    type Output = Vec<TermbaseInfo>;
}

pub struct UpdateTermbase<'a> {
    pub update: &'a TermbaseInfoUpdate,
}

impl Oper for UpdateTermbase<'_> {
    type Output = ();
}

pub struct UpdateTermbaseTermCount<'a> {
    //
    pub id: &'a str,
    pub delta: i32,
}

impl Oper for UpdateTermbaseTermCount<'_> {
    type Output = ();
}

pub struct TouchTermbase<'a> {
    pub id: &'a str,
}

impl Oper for TouchTermbase<'_> {
    type Output = ();
}

pub struct DeleteTermbase<'a> {
    pub id: &'a str,
}

impl Oper for DeleteTermbase<'_> {
    type Output = ();
}
