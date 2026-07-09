//! Diesel entity types for the `t_system_mail` table.

use diesel::prelude::*;
use time::OffsetDateTime;

use crate::model::system_mail::{SystemMailForm, SystemMailInfo};
use crate::part_impl::repo::rdb_impl::schema::t_system_mail;

// ── Queryable / Selectable ─────────────────────────────────────────────────

/// Raw database row for the `t_system_mail` table. Returned by Diesel queries.
#[derive(Queryable, Selectable)]
#[diesel(table_name = t_system_mail)]
pub struct SystemMailRow {
    pub f_id: String,
    pub f_receiver_id: String,

    pub f_title: String,
    pub f_content: String,

    pub f_read: bool,

    pub f_created_at: OffsetDateTime,
}

// ── Insertable ─────────────────────────────────────────────────────────────

/// Insertable struct for creating a new record in the `t_system_mail` table.
#[derive(Insertable)]
#[diesel(table_name = t_system_mail)]
pub struct SystemMailEntry<'a> {
    pub f_id: &'a str,
    pub f_receiver_id: &'a str,

    pub f_title: &'a str,
    pub f_content: &'a str,

    pub f_created_at: OffsetDateTime,
}

// ── Conversions ────────────────────────────────────────────────────────────

impl From<SystemMailRow> for SystemMailInfo {
    fn from(v: SystemMailRow) -> Self {
        SystemMailInfo {
            id: v.f_id,
            receiver_id: v.f_receiver_id,
            read: v.f_read,
            title: v.f_title,
            content: v.f_content,
            created_at: v.f_created_at,
        }
    }
}

impl<'a> From<&'a SystemMailForm> for SystemMailEntry<'a> {
    fn from(form: &'a SystemMailForm) -> Self {
        Self {
            f_id: &form.id,
            f_receiver_id: &form.receiver_id,
            f_title: &form.title,
            f_content: &form.content,
            f_created_at: OffsetDateTime::now_utc(),
        }
    }
}
