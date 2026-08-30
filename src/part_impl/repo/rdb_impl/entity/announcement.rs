//! Diesel entity types for the `t_announcement` table.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::write::announcement::AnnouncementEntry;
use crate::part_impl::repo::rdb_impl::schema::t_announcement;

/// Raw database row for the `t_announcement` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_announcement)]
pub struct AnnouncementInfoRow {
    //
    pub f_id: String,

    pub f_team_id: String,
    pub f_user_id: String,

    pub f_title: String,
    pub f_content: String,

    pub f_created_at: OffsetDateTime,
}

/// Insertable struct for creating a new record in the `t_announcement` table.
#[derive(Insertable)]
#[diesel(table_name = t_announcement)]
pub struct AnnouncementEntryRow<'a> {
    //
    pub f_id: &'a str,

    pub f_team_id: &'a str,
    pub f_user_id: &'a str,

    pub f_title: &'a str,
    pub f_content: &'a str,

    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a AnnouncementEntry> for AnnouncementEntryRow<'a> {
    fn from(entry: &'a AnnouncementEntry) -> Self {
        //
        Self {
            f_id: &entry.id,
            f_team_id: &entry.team_id,
            f_user_id: &entry.user_id,
            f_title: &entry.title,
            f_content: &entry.content,
            f_created_at: OffsetDateTime::now_utc(),
        }
    }
}

impl From<AnnouncementInfoRow> for AnnouncementInfo {
    fn from(row: AnnouncementInfoRow) -> Self {
        //
        Self {
            id: row.f_id,
            team_id: row.f_team_id,
            user_id: row.f_user_id,
            user: None,
            title: row.f_title,
            content: row.f_content,
            created_at: row.f_created_at,
        }
    }
}
