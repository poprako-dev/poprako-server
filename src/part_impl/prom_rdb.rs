//! Diesel-backed prom (promise) adapter.
//!
//! Provides [`RdbPromTransactional`] — a standalone transactional handle
//! for enqueuing deferred actions into the `t_local_message` table.
//! Prom lives in its own module, separate from the repository adapter.

use async_trait::async_trait;
use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use serde_json::Value;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;

use crate::part::prom::{Append, PromTransactional};
use crate::part_impl::repo_rdb::schema;
use crate::part_impl::shared_rdb::result::diesel;
use crate::part_impl::shared_rdb::RdbContext;
use crate::result::RootError;

// ── Entity ─────────────────────────────────────────────────────────────────

#[derive(Insertable)]
#[diesel(table_name = schema::t_local_message)]
struct LocalMessageEntry {
    f_id: String,
    f_topic: String,
    f_status: String,

    f_payload: Value,

    f_visible_at: OffsetDateTime,

    f_created_at: OffsetDateTime,
    f_updated_at: OffsetDateTime,
}

impl LocalMessageEntry {
    fn from_append(step: &Append<'_>, now: OffsetDateTime) -> Result<Self, RootError> {
        let f_payload =
            serde_json::to_value(&step.payload).map_err(|e| RootError::Unrecoverable {
                message: format!("failed to serialize prom payload: {}", e),
            })?;

        Ok(Self {
            f_id: step.id.to_owned(),
            f_topic: step.topic.to_owned(),
            f_status: "pending".to_owned(),
            f_payload,
            f_visible_at: *step.visible_at,
            f_created_at: now,
            f_updated_at: now,
        })
    }
}

// ── Handle type ────────────────────────────────────────────────────────────

pub struct RdbPromTransactional;

// ── PromTransactional impl ──────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Append<'a>, RdbContext> for RdbPromTransactional {
    type Error = RootError;

    async fn advance(&self, context: &mut RdbContext, step: &Append<'a>) -> Result<(), RootError> {
        let now = OffsetDateTime::now_utc();
        let entry = LocalMessageEntry::from_append(step, now)?;

        diesel::insert_into(schema::t_local_message::table)
            .values(&entry)
            .execute(context.conn())
            .await
            .map_err(diesel)?;

        Ok(())
    }
}

impl PromTransactional<RdbContext> for RdbPromTransactional {}
