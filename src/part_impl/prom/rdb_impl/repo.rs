//! Repository for prom task handling and `t_local_message` lifecycle operations.
//!
//! These operation types and [`Step`] implementations are used exclusively
//! by the background actor. They are NOT part of the public [`Prom`]
//! port trait — only producer-side defer operations are exposed through the
//! port system.
//!
//! [`Prom`]: crate::part::prom::Prom

/// RDB prom repository integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{AtLeast, Level, Oper, Step};
use time::OffsetDateTime;
use tracing::instrument;
use uuid::Uuid;

use poprako_rdb_core::RdbConn;

use crate::part::nucl::{ReptRead, Serial};
use crate::part_impl::prom::rdb_impl::entity::{
    LocalMessageRow, LocalMessageStatus,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// Atomically claims at most the available execution capacity, one per topic.
/// Serializable transactions prevent competing claims for the same topic.
#[derive(Oper)]
#[oper(output = Vec<LocalMessageRow>)]
pub struct ClaimPending {
    /// Maximum number of attempts the consumer can start immediately.
    limit: usize,
}

impl ClaimPending {
    /// Reserves no more work than the consumer can execute.
    pub const fn new(limit: usize) -> Self {
        Self { limit }
    }
}

/// Deletes the record owned by a successfully completed attempt.
#[derive(Oper)]
#[oper(output = ())]
pub struct CompleteMessage<'a> {
    // Internal state field `id`.
    /// ID of the local-message row to mark complete.
    id: &'a str,

    /// UUID credential owned by the worker attempt.
    claim_token: Uuid,
}

impl<'a> CompleteMessage<'a> {
    /// Builds an operation that completes the identified worker attempt.
    pub const fn new(id: &'a str, claim_token: Uuid) -> Self {
        Self { id, claim_token }
    }
}

/// Mark a record as dead with an error message.
#[derive(Oper)]
#[oper(output = ())]
pub struct FailMessage<'a> {
    // Internal state field `id`.
    /// ID of the local-message row to mark as failed.
    id: &'a str,

    /// UUID credential owned by the worker attempt.
    claim_token: Uuid,

    /// Error description attached to the failure record.
    error: &'a str,
}

impl<'a> FailMessage<'a> {
    /// Builds an operation that permanently fails the message identified by `id`.
    pub const fn new(id: &'a str, claim_token: Uuid, err_msg: &'a str) -> Self {
        //
        Self {
            id,
            claim_token,
            error: err_msg,
        }
    }
}

/// Reset one failed processing attempt back to pending for a later retry.
#[derive(Oper)]
#[oper(output = ())]
pub struct RetryMessage<'a> {
    // Internal state field `id`.
    /// ID of the local-message row to retry.
    id: &'a str,

    /// UUID credential owned by the worker attempt.
    claim_token: Uuid,

    /// Error description logged from the previous attempt.
    error: &'a str,

    /// Timestamp after which the retry becomes visible for processing.
    visible_at: &'a OffsetDateTime,
    /// Amount consumed from the failure retry budget.
    retry_delta: i64,
}

impl<'a> RetryMessage<'a> {
    /// Builds an operation that schedules the message identified by `id` for retry.
    pub const fn new(
        id: &'a str,
        claim_token: Uuid,
        err_msg: &'a str,
        visible_at: &'a OffsetDateTime,
        retry_delta: i64,
    ) -> Self {
        //
        Self {
            id,
            claim_token,
            error: err_msg,
            visible_at,
            retry_delta,
        }
    }
}

/// Reset expired processing attempts, consuming the shared failure retry budget.
/// The fourth failed attempt becomes dead; waiting does not consume this budget.
#[derive(Oper)]
#[oper(output = ())]
pub struct ResetStuck<'a> {
    /// Cutoff timestamp; any record stuck in Processing before this is reset.
    before: &'a OffsetDateTime,
}

impl<'a> ResetStuck<'a> {
    /// Builds an operation that resets messages stuck before the cutoff.
    pub const fn new(before: &'a OffsetDateTime) -> Self {
        Self { before }
    }
}

/// Deletes dead records after their retention cutoff.
#[derive(Oper)]
#[oper(output = usize)]
pub struct PurgeDead<'a> {
    /// Cutoff timestamp for dead records to purge.
    dead_before: &'a OffsetDateTime,
}

impl<'a> PurgeDead<'a> {
    /// Builds a dead-message purge cutoff.
    pub const fn new(dead_before: &'a OffsetDateTime) -> Self {
        Self { dead_before }
    }
}

// Claims and reads each attempt in one statement, without a stale poll result.
#[instrument(level = "info", skip_all)]
async fn claim_pending(
    conn: &mut RdbConn,
    limit: usize,
) -> BaseRest<Vec<LocalMessageRow>> {
    //
    let limit = i64::try_from(limit).unwrap_or(i64::MAX);

    let rows = diesel::sql_query(
        "WITH candidates AS (
            SELECT DISTINCT ON (pending.f_topic) pending.f_id
            FROM t_local_message AS pending
            WHERE pending.f_status = 'local_message_status:pending'
              AND pending.f_visible_at <= $1
              AND NOT EXISTS (
                  SELECT 1 FROM t_local_message AS processing
                  WHERE processing.f_topic = pending.f_topic
                    AND processing.f_status = 'local_message_status:processing'
              )
            ORDER BY pending.f_topic, pending.f_created_at, pending.f_id
        ), locked AS (
            SELECT message.f_id FROM t_local_message AS message
            JOIN candidates ON candidates.f_id = message.f_id
            ORDER BY message.f_visible_at, message.f_created_at, message.f_id
            LIMIT $2
            FOR UPDATE OF message SKIP LOCKED
        )
        UPDATE t_local_message AS message
        SET f_status = 'local_message_status:processing',
            f_claim_token = gen_random_uuid(),
            f_updated_at = $1
        FROM locked
        WHERE message.f_id = locked.f_id
        RETURNING message.f_id, message.f_topic, message.f_payload,
                  message.f_retried_count, message.f_claim_token, message.f_created_at",
    )
    .bind::<diesel::sql_types::Timestamptz, _>(OffsetDateTime::now_utc())
    .bind::<diesel::sql_types::BigInt, _>(limit)
    .load::<LocalMessageRow>(conn)
    .await
    .map_err(diesel)?;

    accept(rows)
}

// Implements complete message.
#[instrument(level = "info", skip_all)]
async fn complete_message(
    conn: &mut RdbConn,
    id: &str,
    claim_token: Uuid,
) -> BaseRest<()> {
    //
    diesel::delete(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_claim_token.eq(claim_token)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements fail message.
#[instrument(level = "info", skip_all)]
async fn fail_message(
    conn: &mut RdbConn,
    id: &str,
    claim_token: Uuid,
    error: &str,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_claim_token.eq(claim_token)),
    )
    .set((
        t_local_message::f_claim_token.eq(None::<Uuid>),
        t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
        t_local_message::f_last_error.eq(Some(error)),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements retry message.
#[instrument(level = "info", skip_all)]
async fn retry_message(
    conn: &mut RdbConn,
    oper: &RetryMessage<'_>,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(t_local_message::f_id.eq(oper.id))
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_claim_token.eq(oper.claim_token)),
    )
    .set((
        t_local_message::f_claim_token.eq(None::<Uuid>),
        t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
        t_local_message::f_last_error.eq(Some(oper.error)),
        t_local_message::f_retried_count
            .eq(t_local_message::f_retried_count + oper.retry_delta),
        t_local_message::f_visible_at.eq(*oper.visible_at),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Implements reset stuck.
#[instrument(level = "info", skip_all)]
async fn reset_stuck(
    conn: &mut RdbConn,
    before: &OffsetDateTime,
) -> BaseRest<()> {
    //
    diesel::update(
        t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_updated_at.le(*before))
            .filter(t_local_message::f_retried_count.ge(3)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
        t_local_message::f_last_error.eq(Some("processing timeout exceeded")),
        t_local_message::f_claim_token.eq(None::<Uuid>),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    diesel::update(
        t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(t_local_message::f_updated_at.le(*before))
            .filter(t_local_message::f_retried_count.lt(3)),
    )
    .set((
        t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
        t_local_message::f_last_error.eq(Some("processing timeout exceeded")),
        t_local_message::f_retried_count
            .eq(t_local_message::f_retried_count + 1),
        t_local_message::f_claim_token.eq(None::<Uuid>),
        t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
    ))
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(())
}

// Deletes expired dead messages.
#[instrument(level = "info", skip_all)]
async fn purge_dead(
    conn: &mut RdbConn,
    dead_before: &OffsetDateTime,
) -> BaseRest<usize> {
    //
    let purged_count = diesel::delete(
        t_local_message::table
            .filter(
                t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
            )
            .filter(t_local_message::f_updated_at.lt(*dead_before)),
    )
    .execute(conn)
    .await
    .map_err(diesel)?;

    accept(purged_count)
}

/// Queue repository used by the prom background actor.
///
/// Provides atomic claiming, completion, failure, retry, and recovery
/// operations for records in `t_local_message`.
///
/// [`RdbPromActor`]: super::actor::base::RdbPromActor
#[derive(Default)]
pub struct RdbPromRepo;

impl RdbPromRepo {
    /// Builds the local-message queue repository.
    #[must_use]
    pub const fn new() -> Self {
        Self
    }
}

impl<L> Step<ClaimPending, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<Serial>,
{
    // Internal type alias for `Level`.
    type Level = Serial;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ClaimPending,
    ) -> BaseRest<Vec<LocalMessageRow>> {
        claim_pending(context.conn(), oper.limit).await
    }
}

impl<'a, L> Step<CompleteMessage<'a>, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &CompleteMessage<'a>,
    ) -> BaseRest<()> {
        complete_message(context.conn(), oper.id, oper.claim_token).await
    }
}

impl<'a, L> Step<FailMessage<'a>, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &FailMessage<'a>,
    ) -> BaseRest<()> {
        //
        fail_message(context.conn(), oper.id, oper.claim_token, oper.error)
            .await
    }
}

impl<'a, L> Step<RetryMessage<'a>, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &RetryMessage<'a>,
    ) -> BaseRest<()> {
        retry_message(context.conn(), oper).await
    }
}

impl<'a, L> Step<ResetStuck<'a>, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &ResetStuck<'a>,
    ) -> BaseRest<()> {
        reset_stuck(context.conn(), oper.before).await
    }
}

impl<'a, L> Step<PurgeDead<'a>, RdbContext<L>> for RdbPromRepo
where
    L: Level + Send + AtLeast<ReptRead>,
{
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Internal implementation of `step`.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext<L>,
        oper: &PurgeDead<'a>,
    ) -> BaseRest<usize> {
        purge_dead(context.conn(), oper.dead_before).await
    }
}
