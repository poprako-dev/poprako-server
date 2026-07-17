use poprako_orchestra::Oper;

use crate::model::term::{
    TermEntry, TermInfo, TermInfoListSpec, TermInfoUpdate,
};

pub struct CreateTerm<'a> {
    pub entry: &'a TermEntry,
}

impl<'a> Oper for CreateTerm<'a> {
    type Output = TermInfo;
}

pub struct GetTermInfo<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetTermInfo<'a> {
    type Output = TermInfo;
}

pub struct ListTermInfos<'a> {
    pub spec: &'a TermInfoListSpec,
}

impl<'a> Oper for ListTermInfos<'a> {
    type Output = Vec<TermInfo>;
}

pub struct GetTermInfoExcluded<'a> {
    pub id: &'a str,
}

impl<'a> Oper for GetTermInfoExcluded<'a> {
    type Output = TermInfo;
}

pub struct UpdateTerm<'a> {
    pub update: &'a TermInfoUpdate,
}

impl<'a> Oper for UpdateTerm<'a> {
    type Output = ();
}

pub struct DeleteTerm<'a> {
    pub id: &'a str,
}

impl<'a> Oper for DeleteTerm<'a> {
    type Output = ();
}

pub struct DeleteTerms<'a> {
    pub termbase_id: &'a str,
}

impl<'a> Oper for DeleteTerms<'a> {
    type Output = ();
}
