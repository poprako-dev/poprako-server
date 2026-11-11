//! Diesel entity types for the `t_term` table.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::term::TermInfo;
use crate::model::write::term::TermEntry;
use crate::part_impl::repo::rdb_impl::schema::t_term;
use crate::shape::write::term::TermEntryShape;

/// Raw database record for a terminology entry.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_term)]
pub struct TermInfoRecord {
    pub f_id: String,

    pub f_termbase_id: String,

    pub f_source: String,
    pub f_targets: Vec<Option<String>>,
    pub f_comment: Option<String>,

    pub f_creator_id: String,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl From<TermInfoRecord> for TermInfo {
    fn from(record: TermInfoRecord) -> Self {
        Self {
            id: record.f_id,
            termbase_id: record.f_termbase_id,
            source: record.f_source,
            targets: record.f_targets.into_iter().flatten().collect(),
            comment: record.f_comment,
            creator_id: record.f_creator_id,
            created_at: record.f_created_at,
            updated_at: record.f_updated_at,
        }
    }
}

/// Insertable terminology item.
#[derive(Insertable)]
#[diesel(table_name = t_term)]
pub struct TermItem<'a> {
    pub f_id: &'a str,

    pub f_termbase_id: &'a str,

    pub f_source: &'a str,
    pub f_targets: Vec<Option<&'a str>>,
    pub f_comment: Option<&'a str>,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a TermEntry> for TermItem<'a> {
    fn from(entry: &'a TermEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &entry.id,
            f_termbase_id: &entry.termbase_id,
            f_source: entry.trimmed_source(),
            f_targets: entry.trimmed_targets().map(Some).collect(),
            f_comment: entry.trimmed_comment(),
            f_creator_id: &entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}

impl<'a> From<&'a TermEntryShape<'_>> for TermItem<'a> {
    fn from(entry: &'a TermEntryShape<'_>) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &entry.id,
            f_termbase_id: entry.termbase_id,
            f_source: entry.source,
            f_targets: entry
                .targets
                .iter()
                .map(|target| Some(*target))
                .collect(),
            f_comment: entry.comment,
            f_creator_id: entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
