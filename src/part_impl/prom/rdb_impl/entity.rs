//! Diesel entity types for the `t_local_message` table.

use std::io::Write as _;

use diesel::AsExpression;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{IsNull, Output, Result as SerializeResult, ToSql};
use diesel::sql_types::Text;
use time::OffsetDateTime;

use crate::part::prom::Append;
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{RegularError, RegularResult};

/// Lifecycle status of a local message record in the prom queue.
#[derive(Debug, Clone, Copy, PartialEq, Eq, AsExpression)]
#[diesel(sql_type = Text)]
pub enum LocalMessageStatus {
    Pending,
    Processing,
    Completed,
    Dead,
}

impl LocalMessageStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            //
            Self::Pending => "local_message_status:pending",

            Self::Processing => "local_message_status:processing",

            Self::Completed => "local_message_status:completed",

            Self::Dead => "local_message_status:dead",
        }
    }
}

impl ToSql<Text, Pg> for LocalMessageStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> SerializeResult {
        //
        out.write_all(self.as_str().as_bytes())?;

        Ok(IsNull::No)
    }
}

/// Insertable row for the `t_local_message` table.
#[derive(Insertable)]
#[diesel(table_name = t_local_message)]
pub struct LocalMessageEntry<'a> {
    pub f_id: &'a str,
    pub f_topic: &'a str,
    pub f_status: LocalMessageStatus,

    pub f_payload: serde_json::Value,

    pub f_visible_at: OffsetDateTime,

    pub f_created_at: OffsetDateTime,
    pub f_updated_at: OffsetDateTime,
}

impl<'a> LocalMessageEntry<'a> {
    pub fn from_append(
        step: &'a Append<'_>,
        now: OffsetDateTime,
    ) -> RegularResult<Self> {
        //
        let f_payload = serde_json::to_value(&step.payload).map_err(|e| {
            RegularError::Unrecoverable {
                message: format!("failed to serialize prom payload: {}", e),
            }
        })?;

        Ok(Self {
            f_id: step.id,
            f_topic: step.topic,
            f_status: LocalMessageStatus::Pending,
            f_payload,
            f_visible_at: *step.visible_at,
            f_created_at: now,
            f_updated_at: now,
        })
    }
}

/// A row read from `t_local_message` during the poll phase.
#[derive(Debug, Queryable)]
pub struct LocalMessageRow {
    pub f_id: String,
    pub f_topic: String,
    pub f_payload: serde_json::Value,
}
