//! Repository for prom task handling and `t_local_message` lifecycle operations.
//!
//! These operation types and [`Step`] implementations are used exclusively
//! by the background handler. They are NOT part of the public [`Prom`]
//! port trait — only producer-side defer operations are exposed through the
//! port system.
//!
//! [`Prom`]: crate::part::prom::Prom

use poprako_orchestra::{Oper, Step};
use time::OffsetDateTime;
use tracing::instrument;

use crate::part_impl::prom::rdb_impl::entity::{
    LocalMessageRow, LocalMessageStatus,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbContext;
use crate::shared::result::diesel;

/// RDB prom repository integration tests.
#[cfg(all(test, feature = "rdb", feature = "prom_impl"))]
pub mod tests;

// ── Handle ──────────────────────────────────────────────────────────────────

/// Repository used by the prom background handler.
///
/// Owns the application repository used by topic handlers while also providing
/// polling, claiming, completion, failure, retry, and recovery operations for
/// records in `t_local_message`.
///
/// [`RdbPromHandler`]: super::handler::RdbPromHandler
pub struct RdbPromRepo<R> {
    /// Delegate application repository used by topic handlers.
    repo: R,
}

impl<R> RdbPromRepo<R> {
    /// Builds a new prom repository wrapping the given application repo.
    pub fn new(repo: R) -> Self {
        Self { repo }
    }

    /// Returns the application repository used by topic handlers.
    pub fn inner(&self) -> &R {
        &self.repo
    }
}

// ── Operations ──────────────────────────────────────────────────────────────

/// Poll the oldest visible pending record from each topic without processing work.
///
/// A delayed retry is excluded before the per-topic selection, allowing later
/// visible work from the same topic to advance. A processing record blocks only
/// its own topic so separate application instances cannot consume that topic
/// concurrently.
#[derive(Oper)]
#[oper(output = Vec<LocalMessageRow>)]
pub struct PollPending;

/// Try to claim a record (status Pending → Processing).
///
/// Returns `true` if the claim succeeded (i.e. the row was still
/// Pending), `false` if another worker claimed it first.
#[derive(Oper)]
#[oper(output = bool)]
pub struct ClaimPending<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to claim.
    id: &'a str,
    /// Lease observed by the poller.
    lease: i64,
}

impl<'a> ClaimPending<'a> {
    /// Builds an operation that claims the observed message attempt.
    pub fn new(id: &'a str, lease: i64) -> Self {
        Self { id, lease }
    }
}

/// Mark a record as successfully completed.
#[derive(Oper)]
#[oper(output = ())]
pub struct CompleteMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to mark complete.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
}

impl<'a> CompleteMessage<'a> {
    /// Builds an operation that completes the identified worker attempt.
    pub fn new(id: &'a str, lease: i64) -> Self {
        Self { id, lease }
    }
}

/// Mark a record as dead with an error message.
#[derive(Oper)]
#[oper(output = ())]
pub struct FailMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to mark as failed.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
    /// Error description attached to the failure record.
    error: &'a str,
}

impl<'a> FailMessage<'a> {
    /// Builds an operation that permanently fails the message identified by `id`.
    pub fn new(id: &'a str, lease: i64, err_msg: &'a str) -> Self {
        //
        Self {
            id,
            lease,
            error: err_msg,
        }
    }
}

/// Reset one failed processing attempt back to pending for a later retry.
#[derive(Oper)]
#[oper(output = ())]
pub struct RetryMessage<'a> {
    //
    // Internal state field `id`.
    /// ID of the local-message row to retry.
    id: &'a str,
    /// Lease owned by the worker attempt.
    lease: i64,
    /// Error description logged from the previous attempt.
    error: &'a str,
    /// Timestamp after which the retry becomes visible for processing.
    visible_at: &'a OffsetDateTime,
}

impl<'a> RetryMessage<'a> {
    /// Builds an operation that schedules the message identified by `id` for retry.
    pub fn new(
        id: &'a str,
        lease: i64,
        err_msg: &'a str,
        visible_at: &'a OffsetDateTime,
    ) -> Self {
        //
        Self {
            id,
            lease,
            error: err_msg,
            visible_at,
        }
    }
}

/// Reset processing records stuck before a cutoff timestamp.
#[derive(Oper)]
#[oper(output = ())]
pub struct ResetStuck<'a> {
    /// Cutoff timestamp; any record stuck in Processing before this is reset.
    before: &'a OffsetDateTime,
}

impl<'a> ResetStuck<'a> {
    /// Builds an operation that resets messages stuck before the cutoff.
    pub fn new(before: &'a OffsetDateTime) -> Self {
        Self { before }
    }
}

/// Deletes completed and dead records after their independent retention cutoffs.
#[derive(Oper)]
#[oper(output = usize)]
pub struct PurgeCompleted<'a> {
    //
    // Internal state field `completed_before`.
    /// Cutoff timestamp for completed records to purge.
    completed_before: &'a OffsetDateTime,
    /// Cutoff timestamp for dead records to purge.
    dead_before: &'a OffsetDateTime,
}

impl<'a> PurgeCompleted<'a> {
    /// Builds terminal-message purge cutoffs.
    pub fn new(
        completed_before: &'a OffsetDateTime,
        dead_before: &'a OffsetDateTime,
    ) -> Self {
        //
        Self {
            completed_before,
            dead_before,
        }
    }
}

// ── Step impls ──────────────────────────────────────────────────────────────

impl<R> Step<PollPending, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        _oper: &PollPending,
    ) -> BaseRest<Vec<LocalMessageRow>> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        let processing_message =
            diesel::alias!(t_local_message as processing_message);

        let processing_topic = processing_message
            .filter(
                processing_message
                    .field(t_local_message::f_status)
                    .eq(LocalMessageStatus::Processing.as_str()),
            )
            .filter(
                processing_message
                    .field(t_local_message::f_topic)
                    .eq(t_local_message::f_topic),
            );

        let local_message_rows = t_local_message::table
            .filter(
                t_local_message::f_status
                    .eq(LocalMessageStatus::Pending.as_str()),
            )
            .filter(t_local_message::f_visible_at.le(OffsetDateTime::now_utc()))
            .filter(diesel::dsl::not(diesel::dsl::exists(processing_topic)))
            .distinct_on(t_local_message::f_topic)
            .order_by((
                t_local_message::f_topic.asc(),
                t_local_message::f_created_at.asc(),
                t_local_message::f_id.asc(),
            ))
            .select((
                t_local_message::f_id,
                t_local_message::f_topic,
                t_local_message::f_payload,
                t_local_message::f_retried_count,
                t_local_message::f_lease,
            ))
            .load::<LocalMessageRow>(context.conn())
            .await
            .map_err(diesel)?;

        accept(local_message_rows)
    }
}

impl<'a, R> Step<ClaimPending<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ClaimPending<'a>,
    ) -> BaseRest<bool> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        let updated = diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(oper.id))
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Pending.as_str()),
                )
                .filter(t_local_message::f_lease.eq(oper.lease)),
        )
        .set((
            t_local_message::f_status
                .eq(LocalMessageStatus::Processing.as_str()),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(updated > 0)
    }
}

impl<'a, R> Step<CompleteMessage<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CompleteMessage<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(oper.id))
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_lease.eq(oper.lease)),
        )
        .set((
            t_local_message::f_status
                .eq(LocalMessageStatus::Completed.as_str()),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(())
    }
}

impl<'a, R> Step<FailMessage<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FailMessage<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(oper.id))
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_lease.eq(oper.lease)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
            t_local_message::f_last_error.eq(Some(oper.error)),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(())
    }
}

impl<'a, R> Step<RetryMessage<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &RetryMessage<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table
                .filter(t_local_message::f_id.eq(oper.id))
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_lease.eq(oper.lease)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
            t_local_message::f_last_error.eq(Some(oper.error)),
            t_local_message::f_retried_count
                .eq(t_local_message::f_retried_count + 1),
            t_local_message::f_visible_at.eq(*oper.visible_at),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(())
    }
}

impl<'a, R> Step<ResetStuck<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ResetStuck<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_updated_at.le(*oper.before))
                .filter(t_local_message::f_lease.ge(3)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
            t_local_message::f_last_error
                .eq(Some("processing timeout exceeded")),
            t_local_message::f_lease.eq(t_local_message::f_lease + 1),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        diesel::update(
            t_local_message::table
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_updated_at.le(*oper.before))
                .filter(t_local_message::f_lease.lt(3)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
            t_local_message::f_lease.eq(t_local_message::f_lease + 1),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(())
    }
}

impl<'a, R> Step<PurgeCompleted<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &PurgeCompleted<'a>,
    ) -> BaseRest<usize> {
        //
        // Internal implementation detail.
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        let (expired_completed, expired_dead) = (
            t_local_message::f_status
                .eq(LocalMessageStatus::Completed.as_str())
                .and(t_local_message::f_updated_at.lt(*oper.completed_before)),
            t_local_message::f_status
                .eq(LocalMessageStatus::Dead.as_str())
                .and(t_local_message::f_updated_at.lt(*oper.dead_before)),
        );

        let purged_count = diesel::delete(
            t_local_message::table.filter(expired_completed.or(expired_dead)),
        )
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        accept(purged_count)
    }
}
