//! Diesel-backed prom (promise) adapter.
//!
//! Provides [`RdbPromTransactional`] — a standalone transactional handle
//! for enqueuing deferred actions into the `t_local_message` table.
//! Prom lives in its own module, separate from the repository adapter.

mod handler;

use std::io::Write as _;
use std::sync::Arc;

use async_trait::async_trait;
use diesel::AsExpression;
use diesel::pg::Pg;
use diesel::prelude::*;
use diesel::serialize::{Output, ToSql};
use diesel::sql_types::Text;
use diesel_async::RunQueryDsl;
use serde_json::Value;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part::image::ImagePool;
use crate::part::prom::{Append, Prom};
use crate::part_impl::drive_rdb::RdbDrive;
use crate::part_impl::prom_rdb::handler::RdbPromHandler;
use crate::part_impl::rdb_core::result::diesel;
use crate::part_impl::rdb_core::{RdbContext, RdbCore};
use crate::part_impl::repo_rdb::RdbRepo;
use crate::part_impl::repo_rdb::schema::t_local_message;
use crate::result::{RegularError, RegularResult};

/// Spawns the prom background worker. Returns immediately; the worker
/// runs on a tokio background task.
pub fn spawn_handler(
    core: RdbCore,
    image_pool: impl ImagePool + Send + Sync + 'static,
) {
    let handler = RdbPromHandler::new(
        core.clone(),
        RdbDrive::new(core.clone()),
        Arc::new(RdbRepo::new(core)),
        RdbProm,
        image_pool,
    );

    tokio::spawn(async move {
        handler.run().await;
    });
}

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
    fn to_sql<'b>(
        &'b self,
        out: &mut Output<'b, '_, Pg>,
    ) -> diesel::serialize::Result {
        out.write_all(self.as_str().as_bytes())?;
        Ok(diesel::serialize::IsNull::No)
    }
}

#[derive(Insertable)]
#[diesel(table_name = t_local_message)]
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
    fn from_append(
        step: &'a Append<'_>,
        now: OffsetDateTime,
    ) -> RegularResult<Self> {
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

// ── Handle type ────────────────────────────────────────────────────────────

pub struct RdbProm;

// ── PromTransactional impl ──────────────────────────────────────────────────

#[async_trait]
impl<'a> Advance<Append<'a>, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &Append<'a>,
    ) -> RegularResult<()> {
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

// ── Prom-consumption Steps ─────────────────────────────────────────────────
//
// These Step types carry the internal t_local_message read/update
// operations. They are NOT exposed through any port trait — only
// Append is exposed via Prom<C>. Advance impls for these Steps live
// on RdbProm and are used exclusively by RdbPromHandler.

/// A row read from t_local_message during the poll phase.
#[derive(Debug, Queryable)]
pub struct LocalMessageRow {
    pub f_id: String,
    pub f_topic: String,
    pub f_payload: serde_json::Value,
}

/// Poll for pending prom records that are visible now.
pub struct PollPending;

impl Step for PollPending {
    type Output = Vec<LocalMessageRow>;
}

/// Try to claim a record (status Pending → Processing).
///
/// Returns `true` if the claim succeeded (i.e. the row was still
/// Pending), `false` if another worker claimed it first.
pub struct ClaimStep<'a> {
    pub id: &'a str,
}

impl Step for ClaimStep<'_> {
    type Output = bool;
}

/// Mark a record as successfully completed.
pub struct CompleteStep<'a> {
    pub id: &'a str,
}

impl Step for CompleteStep<'_> {
    type Output = ();
}

/// Mark a record as dead with an error message.
pub struct FailStep<'a> {
    pub id: &'a str,
    pub error: &'a str,
}

impl Step for FailStep<'_> {
    type Output = ();
}

// ── Advance impls for internal Steps ───────────────────────────────────────

const BATCH_SIZE: i64 = 10;

#[async_trait]
impl Advance<PollPending, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        _step: &PollPending,
    ) -> RegularResult<Vec<LocalMessageRow>> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Pending.as_str()),
            )
            .filter(t_local_message::f_visible_at.le(OffsetDateTime::now_utc()))
            .order_by(t_local_message::f_created_at.asc())
            .limit(BATCH_SIZE)
            .select((
                t_local_message::f_id,
                t_local_message::f_topic,
                t_local_message::f_payload,
            ))
            .load::<LocalMessageRow>(context.conn())
            .await
            .map_err(diesel)
    }
}

#[async_trait]
impl<'a> Advance<ClaimStep<'a>, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ClaimStep<'a>,
    ) -> RegularResult<bool> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        let updated = diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(step.id))
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Pending.as_str()),
                ),
        )
        .set((
            t_local_message::f_status
                .eq(LocalMessageStatus::Processing.as_str()),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(updated > 0)
    }
}

#[async_trait]
impl<'a> Advance<CompleteStep<'a>, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &CompleteStep<'a>,
    ) -> RegularResult<()> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table.filter(t_local_message::f_id.eq(step.id)),
        )
        .set((
            t_local_message::f_status
                .eq(LocalMessageStatus::Completed.as_str()),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<FailStep<'a>, RdbContext> for RdbProm {
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FailStep<'a>,
    ) -> RegularResult<()> {
        use diesel::prelude::*;
        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table.filter(t_local_message::f_id.eq(step.id)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
            t_local_message::f_last_error.eq(Some(step.error)),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}
