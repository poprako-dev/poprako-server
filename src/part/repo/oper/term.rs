use poprako_orchestra::Oper;

use crate::model::term::{
    TermEntry, TermInfo, TermInfoListSpec, TermInfoUpdate,
};

/// Creates a term.
pub struct CreateTerm<'a> {
    /// The term entry to insert.
    pub entry: &'a TermEntry,
}

impl Oper for CreateTerm<'_> {
    // Operation output type.
    type Output = TermInfo;
}

/// Looks up a term by identifier.
pub struct GetTermInfo<'a> {
    /// The term id.
    pub id: &'a str,
}

impl Oper for GetTermInfo<'_> {
    // Operation output type.
    type Output = TermInfo;
}

/// Lists term infos matching a filter spec.
pub struct ListTermInfos<'a> {
    /// The specification for filtering listed terms.
    pub spec: &'a TermInfoListSpec,
}

impl Oper for ListTermInfos<'_> {
    // Operation output type.
    type Output = Vec<TermInfo>;
}

/// Looks up a term by identifier, matching deleted rows as well.
pub struct GetTermInfoExcluded<'a> {
    /// The term id.
    pub id: &'a str,
}

impl Oper for GetTermInfoExcluded<'_> {
    // Operation output type.
    type Output = TermInfo;
}

/// Locks a term row.
pub struct LockTerm<'a> {
    /// The term id.
    pub id: &'a str,
}

impl Oper for LockTerm<'_> {
    // Operation output type.
    type Output = ();
}

/// Updates a term.
pub struct UpdateTerm<'a> {
    /// The update payload for the term.
    pub update: &'a TermInfoUpdate,
}

impl Oper for UpdateTerm<'_> {
    // Operation output type.
    type Output = ();
}

/// Deletes one term.
pub struct DeleteTerm<'a> {
    /// The term id.
    pub id: &'a str,
}

impl Oper for DeleteTerm<'_> {
    // Operation output type.
    type Output = ();
}

/// Deletes all terms for a termbase.
pub struct DeleteTerms<'a> {
    /// The termbase id whose terms to delete.
    pub termbase_id: &'a str,
}

impl Oper for DeleteTerms<'_> {
    // Operation output type.
    type Output = ();
}
