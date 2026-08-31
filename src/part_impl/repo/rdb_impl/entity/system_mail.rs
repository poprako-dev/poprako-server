//! Diesel entity types for the `t_system_mail` table.

use diesel::{Insertable, Queryable, Selectable};
use time::OffsetDateTime;

use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part_impl::repo::rdb_impl::schema::t_system_mail;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_system_mail` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_system_mail)]
pub struct SystemMailInfoRow {
    //
    pub f_id: String,

    pub f_receiver_id: String,

    pub f_title: String,
    pub f_content: String,

    pub f_read: bool,

    pub f_created_at: OffsetDateTime,
}

impl From<SystemMailInfoRow> for SystemMailInfo {
    fn from(v: SystemMailInfoRow) -> Self {
        //
        Self {
            id: v.f_id,
            receiver_id: v.f_receiver_id,
            is_read: v.f_read,
            title: v.f_title,
            content: v.f_content,
            created_at: v.f_created_at,
        }
    }
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_system_mail` table.
#[derive(Insertable)]
#[diesel(table_name = t_system_mail)]
pub struct SystemMailEntryRow<'a> {
    //
    pub f_id: &'a str,

    pub f_receiver_id: &'a str,

    pub f_title: &'a str,
    pub f_content: &'a str,

    pub f_created_at: OffsetDateTime,
}

impl<'a> From<&'a SystemMailEntry> for SystemMailEntryRow<'a> {
    fn from(entry: &'a SystemMailEntry) -> Self {
        //
        Self {
            f_id: &entry.id,
            f_receiver_id: &entry.receiver_id,
            f_title: &entry.title,
            f_content: &entry.content,
            f_created_at: OffsetDateTime::now_utc(),
        }
    }
}
