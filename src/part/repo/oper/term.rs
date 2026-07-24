use poprako_orchestra::Oper;

use crate::model::term::{
    TermEntry, TermInfo, TermInfoListSpec, TermInfoUpdate,
};

pub struct CreateTerm<'a> {
    pub entry: &'a TermEntry,
}

impl Oper for CreateTerm<'_> {
    type Output = TermInfo;
}

pub struct GetTermInfo<'a> {
    pub id: &'a str,
}

impl Oper for GetTermInfo<'_> {
    type Output = TermInfo;
}

pub struct ListTermInfos<'a> {
    pub spec: &'a TermInfoListSpec,
}

impl Oper for ListTermInfos<'_> {
    type Output = Vec<TermInfo>;
}

pub struct GetTermInfoExcluded<'a> {
    pub id: &'a str,
}

impl Oper for GetTermInfoExcluded<'_> {
    type Output = TermInfo;
}

/// Locks a term row.
pub struct LockTerm<'a> {
    pub id: &'a str,
}

impl Oper for LockTerm<'_> {
    type Output = ();
}

pub struct UpdateTerm<'a> {
    pub update: &'a TermInfoUpdate,
}

impl Oper for UpdateTerm<'_> {
    type Output = ();
}

pub struct DeleteTerm<'a> {
    pub id: &'a str,
}

impl Oper for DeleteTerm<'_> {
    type Output = ();
}

pub struct DeleteTerms<'a> {
    pub termbase_id: &'a str,
}

impl Oper for DeleteTerms<'_> {
    type Output = ();
}
