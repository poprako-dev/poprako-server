//! Diesel entity types for the `t_term` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::term::TermInfo;
use crate::model::write::term::TermEntry;
use crate::part_impl::repo::rdb_impl::schema::t_term;

/// Raw database row for a terminology entry.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_term)]
pub struct TermInfoRow {
    pub f_id: String,

    pub f_termbase_id: String,

    pub f_source: String,
    pub f_targets: Vec<Option<String>>,
    pub f_comment: Option<String>,

    pub f_creator_id: String,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl From<TermInfoRow> for TermInfo {
    fn from(row: TermInfoRow) -> Self {
        //
        Self {
            id: row.f_id,
            termbase_id: row.f_termbase_id,
            source: row.f_source,
            targets: row.f_targets.into_iter().flatten().collect(),
            comment: row.f_comment,
            creator_id: row.f_creator_id,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        }
    }
}

/// Insertable terminology-entry row.
#[derive(Insertable)]
#[diesel(table_name = t_term)]
pub struct TermEntryRow<'a> {
    pub f_id: &'a str,

    pub f_termbase_id: &'a str,

    pub f_source: &'a str,
    pub f_targets: Vec<Option<&'a str>>,
    pub f_comment: Option<&'a str>,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a TermEntry> for TermEntryRow<'a> {
    fn from(term_entry: &'a TermEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &term_entry.id,
            f_termbase_id: &term_entry.termbase_id,
            f_source: &term_entry.source,
            f_targets: term_entry
                .targets
                .iter()
                .map(|target| Some(target.as_str()))
                .collect(),
            f_comment: term_entry.comment.as_deref(),
            f_creator_id: &term_entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
