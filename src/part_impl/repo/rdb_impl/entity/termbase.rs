//! Diesel entity types for the `t_termbase` table.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::termbase::TermbaseEntry;
use crate::part_impl::repo::rdb_impl::numeric::usize_from_i32;
use crate::part_impl::repo::rdb_impl::schema::t_termbase;
use crate::result::BaseError;

/// Raw database record for a terminology base.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_termbase)]
pub struct TermbaseInfoRecord {
    pub f_id: String,

    pub f_team_id: Option<String>,
    pub f_comic_id: Option<String>,

    pub f_name: String,
    pub f_description: Option<String>,

    pub f_term_count: i32,

    pub f_creator_id: String,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl TryFrom<TermbaseInfoRecord> for TermbaseInfo {
    type Error = BaseError;

    fn try_from(record: TermbaseInfoRecord) -> Result<Self, Self::Error> {
        //
        Ok(Self {
            id: record.f_id,
            team_id: record.f_team_id,
            comic_id: record.f_comic_id,
            name: record.f_name,
            description: record.f_description,
            term_count: usize_from_i32(
                record.f_term_count,
                "t_termbase.f_term_count",
            )?,
            creator_id: record.f_creator_id,
            created_at: record.f_created_at,
            updated_at: record.f_updated_at,
        })
    }
}

/// Insertable terminology-base record.
#[derive(Insertable)]
#[diesel(table_name = t_termbase)]
pub struct TermbaseItem<'a> {
    pub f_id: &'a str,

    pub f_team_id: Option<&'a str>,
    pub f_comic_id: Option<&'a str>,

    pub f_name: &'a str,
    pub f_description: Option<&'a str>,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a TermbaseEntry> for TermbaseItem<'a> {
    fn from(entry: &'a TermbaseEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &entry.id,
            f_team_id: entry.team_id.as_deref(),
            f_comic_id: entry.comic_id.as_deref(),
            f_name: entry.trimmed_name(),
            f_description: entry.trimmed_description(),
            f_creator_id: &entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
