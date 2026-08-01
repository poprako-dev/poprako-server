//! Diesel entity types for the `t_comment` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::read::proj::comment::CommentInfo;
use crate::model::write::comment::CommentEntry;
use crate::part_impl::repo::rdb_impl::schema::t_comment;

/// Raw database row for the `t_comment` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_comment)]
pub struct CommentInfoRow {
    //
    pub f_id: String,

    pub f_team_id: String,
    pub f_user_id: String,

    pub f_content: String,

    pub f_created_at: OffsetDateTime,
}

impl From<CommentInfoRow> for CommentInfo {
    fn from(row: CommentInfoRow) -> Self {
        Self {
            id: row.f_id,
            team_id: row.f_team_id,
            user_id: row.f_user_id,
            user: None,
            content: row.f_content,
            created_at: row.f_created_at,
        }
    }
}

/// Insertable struct for creating a new record in the `t_comment` table.
#[derive(Insertable)]
#[diesel(table_name = t_comment)]
pub struct CommentEntryRow<'a> {
    //
    pub f_id: &'a str,

    pub f_team_id: &'a str,
    pub f_user_id: &'a str,

    pub f_content: &'a str,

    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a CommentEntry> for CommentEntryRow<'a> {
    fn from(entry: &'a CommentEntry) -> Self {
        Self {
            f_id: &entry.id,
            f_team_id: &entry.team_id,
            f_user_id: &entry.user_id,
            f_content: &entry.content,
            f_created_at: OffsetDateTime::now_utc(),
        }
    }
}
