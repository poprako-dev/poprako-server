use poprako_orchestra::Oper;

use crate::model::term::{
    TermEntry, TermInfo, TermInfoListSpec, TermInfoUpdate,
};

/// Creates a term.
#[derive(Oper)]
#[oper(output = TermInfo)]
pub struct CreateTerm<'a> {
    /// The term entry to insert.
    pub entry: &'a TermEntry,
}

/// Looks up a term by identifier.
#[derive(Oper)]
#[oper(output = TermInfo)]
pub struct GetTermInfo<'a> {
    /// The term id.
    pub id: &'a str,
}

/// Lists term infos matching a filter spec.
#[derive(Oper)]
#[oper(output = Vec<TermInfo>)]
pub struct ListTermInfos<'a> {
    /// The specification for filtering listed terms.
    pub spec: &'a TermInfoListSpec,
}

/// Looks up a term by identifier, matching deleted rows as well.
#[derive(Oper)]
#[oper(output = TermInfo)]
pub struct GetTermInfoExcluded<'a> {
    /// The term id.
    pub id: &'a str,
}

/// Locks a term row.
#[derive(Oper)]
#[oper(output = ())]
pub struct LockTerm<'a> {
    /// The term id.
    pub id: &'a str,
}

/// Updates a term.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpdateTerm<'a> {
    /// The update payload for the term.
    pub update: &'a TermInfoUpdate,
}

/// Deletes one term.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteTerm<'a> {
    /// The term id.
    pub id: &'a str,
}

/// Deletes all terms for a termbase.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteTerms<'a> {
    /// The termbase id whose terms to delete.
    pub termbase_id: &'a str,
}
