//! Diesel entity types for the `t_termbase` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::termbase::TermbaseInfo;
use crate::model::write::termbase::TermbaseEntry;
use crate::part_impl::repo::rdb_impl::schema::t_termbase;

/// Raw database row for a terminology base.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_termbase)]
pub struct TermbaseRow {
    //
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

impl From<TermbaseRow> for TermbaseInfo {
    fn from(row: TermbaseRow) -> Self {
        Self {
            id: row.f_id,
            team_id: row.f_team_id,
            comic_id: row.f_comic_id,
            name: row.f_name,
            description: row.f_description,
            term_count: row.f_term_count,
            creator_id: row.f_creator_id,
            created_at: row.f_created_at,
            updated_at: row.f_updated_at,
        }
    }
}

/// Insertable terminology-base row.
#[derive(Insertable)]
#[diesel(table_name = t_termbase)]
pub struct TermbaseRowEntry<'a> {
    //
    pub f_id: &'a str,

    pub f_team_id: Option<&'a str>,
    pub f_comic_id: Option<&'a str>,

    pub f_name: &'a str,
    pub f_description: Option<&'a str>,

    pub f_creator_id: &'a str,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> From<&'a TermbaseEntry> for TermbaseRowEntry<'a> {
    fn from(termbase_entry: &'a TermbaseEntry) -> Self {
        //
        let now = OffsetDateTime::now_utc();

        Self {
            f_id: &termbase_entry.id,
            f_team_id: termbase_entry.team_id.as_deref(),
            f_comic_id: termbase_entry.comic_id.as_deref(),
            f_name: &termbase_entry.name,
            f_description: termbase_entry.description.as_deref(),
            f_creator_id: &termbase_entry.creator_id,
            f_created_at: now,
            f_updated_at: now,
        }
    }
}
