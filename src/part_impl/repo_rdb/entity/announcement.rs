//! Diesel entity types for the `t_announcement` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::announcement::{AnnouncementForm, AnnouncementInfo};
use crate::part_impl::repo_rdb::schema::t_announcement;

#[derive(Queryable, Selectable)]
#[diesel(table_name = t_announcement)]
pub struct AnnouncementRow {
    pub f_id: String,

    pub f_team_id: String,
    pub f_user_id: String,

    pub f_title: String,
    pub f_content: String,

    pub f_created_at: OffsetDateTime,
}

#[derive(Insertable)]
#[diesel(table_name = t_announcement)]
pub struct AnnouncementEntry<'a> {
    pub f_id: &'a str,

    pub f_team_id: &'a str,
    pub f_user_id: &'a str,

    pub f_title: &'a str,
    pub f_content: &'a str,

    pub f_created_at: OffsetDateTime,
}

impl From<AnnouncementRow> for AnnouncementInfo {
    fn from(row: AnnouncementRow) -> Self {
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

impl<'a> From<&'a AnnouncementForm> for AnnouncementEntry<'a> {
    fn from(form: &'a AnnouncementForm) -> Self {
        Self {
            f_id: &form.id,
            f_team_id: &form.team_id,
            f_user_id: &form.user_id,
            f_title: &form.title,
            f_content: &form.content,
            f_created_at: OffsetDateTime::now_utc(),
        }
    }
}
