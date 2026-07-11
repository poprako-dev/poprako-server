//! Repository for prom task handling and `t_local_message` lifecycle operations.
//!
//! These step types and [`Advance`] implementations are used exclusively
//! by the background handler. They are NOT part of the public [`Prom`]
//! port trait — only [`Append`] is exposed through the port system.
//!
//! [`Prom`]: crate::part::prom::Prom

use async_trait::async_trait;
use time::OffsetDateTime;

use poprako_transactional::advance::Advance;
use poprako_transactional::step::Step;

use crate::part_impl::prom::rdb_impl::entity::{
    LocalMessageRow, LocalMessageStatus,
};
use crate::part_impl::repo::rdb_impl::schema::t_local_message;
use crate::part_impl::shared::RdbContext;
use crate::part_impl::shared::result::diesel;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

// ── Handle ──────────────────────────────────────────────────────────────────

/// Repository used by the prom background handler.
///
/// Owns the application repository used by topic handlers while also providing
/// polling, claiming, completion, failure, retry, and recovery operations for
/// records in `t_local_message`.
///
/// [`RdbPromHandler`]: super::handler::RdbPromHandler
pub struct RdbPromRepo<R> {
    repo: R,
}

impl<R> RdbPromRepo<R> {
    pub fn new(repo: R) -> Self {
        Self { repo }
    }
}

#[async_trait]
impl<R> DeriveTransactional for RdbPromRepo<R>
where
    R: DeriveTransactional + Send + Sync,
{
    type Transactional = R::Transactional;

    async fn derive_transactional(&self) -> Self::Transactional {
        DeriveTransactional::derive_transactional(&self.repo).await
    }
}

// ── Steps ───────────────────────────────────────────────────────────────────

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
    id: &'a str,
}

impl<'a> ClaimStep<'a> {
    /// Builds a step that claims the message identified by `id`.
    pub fn new(id: &'a str) -> Self {
        Self { id }
    }
}

impl Step for ClaimStep<'_> {
    type Output = bool;
}

/// Mark a record as successfully completed.
pub struct CompleteStep<'a> {
    id: &'a str,
}

impl<'a> CompleteStep<'a> {
    /// Builds a step that completes the message identified by `id`.
    pub fn new(id: &'a str) -> Self {
        Self { id }
    }
}

impl Step for CompleteStep<'_> {
    type Output = ();
}

/// Mark a record as dead with an error message.
pub struct FailStep<'a> {
    id: &'a str,
    error: &'a str,
}

impl<'a> FailStep<'a> {
    /// Builds a step that permanently fails the message identified by `id`.
    pub fn new(id: &'a str, error: &'a str) -> Self {
        Self { id, error }
    }
}

impl Step for FailStep<'_> {
    type Output = ();
}

/// Reset one failed processing attempt back to pending for a later retry.
pub struct RetryStep<'a> {
    id: &'a str,
    error: &'a str,
    visible_at: &'a OffsetDateTime,
}

impl<'a> RetryStep<'a> {
    /// Builds a step that schedules the message identified by `id` for retry.
    pub fn new(
        id: &'a str,
        error: &'a str,
        visible_at: &'a OffsetDateTime,
    ) -> Self {
        Self {
            id,
            error,
            visible_at,
        }
    }
}

impl Step for RetryStep<'_> {
    type Output = ();
}

/// Reset processing records stuck before a cutoff timestamp.
pub struct ResetStuckStep<'a> {
    before: &'a OffsetDateTime,
}

impl<'a> ResetStuckStep<'a> {
    /// Builds a step that resets messages stuck before the cutoff.
    pub fn new(before: &'a OffsetDateTime) -> Self {
        Self { before }
    }
}

impl Step for ResetStuckStep<'_> {
    type Output = ();
}

// ── Advance impls ───────────────────────────────────────────────────────────

/// Maximum number of pending records to poll in a single batch.
const BATCH_SIZE: i64 = 10;

#[async_trait]
impl<R> Advance<PollPending, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        _step: &PollPending,
    ) -> RegularResult<Vec<LocalMessageRow>> {
        //
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        let local_message_rows: Vec<LocalMessageRow> = t_local_message::table
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
            .load(context.conn())
            .await
            .map_err(diesel)?;

        Ok(local_message_rows)
    }
}

#[async_trait]
impl<'a, R> Advance<ClaimStep<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ClaimStep<'a>,
    ) -> RegularResult<bool> {
        //
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
impl<'a, R> Advance<CompleteStep<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &CompleteStep<'a>,
    ) -> RegularResult<()> {
        //
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
impl<'a, R> Advance<FailStep<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &FailStep<'a>,
    ) -> RegularResult<()> {
        //
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

#[async_trait]
impl<'a, R> Advance<RetryStep<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &RetryStep<'a>,
    ) -> RegularResult<()> {
        //
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table.filter(t_local_message::f_id.eq(step.id)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Pending.as_str()),
            t_local_message::f_last_error.eq(Some(step.error)),
            t_local_message::f_retried_count
                .eq(t_local_message::f_retried_count + 1),
            t_local_message::f_visible_at.eq(*step.visible_at),
            t_local_message::f_updated_at.eq(OffsetDateTime::now_utc()),
        ))
        .execute(context.conn())
        .await
        .map_err(diesel)?;

        Ok(())
    }
}

#[async_trait]
impl<'a, R> Advance<ResetStuckStep<'a>, RdbContext> for RdbPromRepo<R>
where
    R: Sync,
{
    type Error = RegularError;

    async fn advance(
        &self,
        context: &mut RdbContext,
        step: &ResetStuckStep<'a>,
    ) -> RegularResult<()> {
        //
        use diesel::prelude::*;

        use diesel_async::RunQueryDsl;

        diesel::update(
            t_local_message::table
                .filter(
                    t_local_message::f_status
                        .eq(LocalMessageStatus::Processing.as_str()),
                )
                .filter(t_local_message::f_updated_at.le(*step.before))
                .filter(t_local_message::f_lease.ge(3)),
        )
        .set((
            t_local_message::f_status.eq(LocalMessageStatus::Dead.as_str()),
            t_local_message::f_last_error
                .eq(Some("processing timeout exceeded")),
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
                .filter(t_local_message::f_updated_at.le(*step.before))
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

        Ok(())
    }
}
