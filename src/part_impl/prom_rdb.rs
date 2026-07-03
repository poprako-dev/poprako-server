//! Diesel-backed prom (promise) adapter.
//!
//! Provides [`RdbPromTransactional`] — a standalone transactional handle
//! for enqueuing deferred actions into the `t_local_message` table.
//! Prom lives in its own module, separate from the repository adapter.

use async_trait::async_trait;
use diesel::AsExpression;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;
use serde_json::Value;
use std::io::Write as _;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::prom::{Append, Prom};
use crate::part_impl::rdb_core::RdbContext;
use crate::part_impl::rdb_core::result::diesel;
use crate::part_impl::repo_rdb::schema::{self, t_local_message};
use crate::result::{RegularError, RegularResult};

// ── Entity ─────────────────────────────────────────────────────────────────

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
            Self::Pending => "local_message_status:pending",
            Self::Processing => "local_message_status:processing",
            Self::Completed => "local_message_status:completed",
            Self::Dead => "local_message_status:dead",
        }
    }
}

impl ToSql<Text, Pg> for LocalMessageStatus {
    fn to_sql<'b>(&'b self, out: &mut Output<'b, '_, Pg>) -> diesel::serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

#[derive(Insertable)]
#[diesel(table_name = schema::t_local_message)]
struct LocalMessageEntry<'a> {
    f_id: &'a str,
    f_topic: &'a str,
    f_status: LocalMessageStatus,

    f_payload: Value,

    f_visible_at: OffsetDateTime,

    f_created_at: OffsetDateTime,
    f_updated_at: OffsetDateTime,
}

impl<'a> LocalMessageEntry<'a> {
    fn from_append(step: &'a Append<'_>, now: OffsetDateTime) -> RegularResult<Self> {
        let f_payload =
            serde_json::to_value(&step.payload).map_err(|e| RegularError::Unrecoverable {
                message: format!("failed to serialize prom payload: {}", e),
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

// ── Handle type ────────────────────────────────────────────────────────────

pub struct RdbProm;

// ── PromTransactional impl ──────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Append<'a>, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(&self, context: &mut RdbContext, step: &Append<'a>) -> RegularResult<()> {
        let now = OffsetDateTime::now_utc();
        let entry = LocalMessageEntry::from_append(step, now)?;

        diesel::insert_into(t_local_message::table)
            .values(&entry)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

impl Prom<RdbContext> for RdbProm {}
