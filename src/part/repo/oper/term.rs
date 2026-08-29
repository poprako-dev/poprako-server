use poprako_orchestra::Oper;

use crate::model::read::proj::term::TermInfo;
use crate::model::write::term::{TermEntry, TermRepl};

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

/// Lists term infos selected by a paged query or terminology base.
#[derive(Oper)]
#[oper(output = Vec<TermInfo>)]
pub enum ListTermInfos<'a> {
    /// Lists one page of terms with an optional source filter.
    Query {
        /// The terminology-base identifier.
        termbase_id: &'a str,
        /// Optional normalized substring matched against term sources.
        fuzzy_source: Option<&'a str>,
        /// Number of matching terms to skip.
        offset: u32,
        /// Maximum number of matching terms to return.
        limit: u32,
    },

    /// Lists every term belonging to one terminology base.
    Termbase {
        /// The terminology-base identifier.
        termbase_id: &'a str,
    },
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
    pub update: &'a TermRepl,
}

/// Applies imported terminology inserts and replacements together.
#[derive(Oper)]
#[oper(output = ())]
pub struct UpsertTerms<'a> {
    /// New terminology entries to insert.
    pub entries: &'a [TermEntry],
    /// Existing terminology entries to replace.
    pub updates: &'a [TermRepl],
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
