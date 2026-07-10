//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using the Advance pattern for local-message lifecycle.
//!
//! Topic dispatch routes to [`image`].

use std::marker::PhantomData;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration as StdDuration;

use time::{Duration, OffsetDateTime};
use tokio::sync::oneshot::{
    Receiver as OneshotReceiver, Sender as OneshotSender,
};
use tokio::time::sleep;
use tracing::{Level, instrument};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::part::image::ImagePool;
use crate::part::prom::task::{IMAGE_TOPIC, ImageTask};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::team::TeamRepoTransactional;
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::user::UserRepoTransactional;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimStep, CompleteStep, FailStep, LocalMessageRepo, PollPending,
    ResetStuckStep, RetryStep,
};
use crate::part_impl::shared::{RdbContext, RdbCore};
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

/// Prom image event handler.
mod image;

/// Interval between successive poll cycles in the prom background worker.
const POLL_INTERVAL: StdDuration = StdDuration::from_secs(5);
const RETRY_DELAY: Duration = Duration::minutes(5);
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);

pub enum TaskOutcome {
    Complete,
    Retry(String),
    Dead(String),
}

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromHandler<D, R, I> {
    core: RdbCore,
    drive: D,

    repo: Arc<R>,

    local_message_repo: LocalMessageRepo,
    image_pool: I,

    shutdown_recv: OneshotReceiver<()>,
    done_send: OneshotSender<()>,
    accepting: Arc<AtomicBool>,

    _p: PhantomData<RdbContext>,
}

impl<D, R, I> RdbPromHandler<D, R, I>
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional
        + ComicRepo<RdbContext>
        + WorksetRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + PageRepo<RdbContext>
        + AssignmentRepo<RdbContext>
        + AssignmentInvitationRepo<RdbContext>
        + UnitRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<RdbContext>
            + WorksetRepoTransactional<RdbContext>
            + ChapterRepoTransactional<RdbContext>
            + PageRepoTransactional<RdbContext>
            + TeamRepoTransactional<RdbContext>
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + UserRepoTransactional<RdbContext>
            + Send
            + Sync,
    I: ImagePool + Send + Sync + 'static,
{
    pub fn new(
        core: RdbCore,
        drive: D,
        repo: Arc<R>,
        local_message_repo: LocalMessageRepo,
        image_pool: I,
        shutdown_recv: OneshotReceiver<()>,
        done_send: OneshotSender<()>,
        accepting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            core,
            drive,
            repo,
            local_message_repo,
            image_pool,
            shutdown_recv,
            done_send,
            accepting,
            _p: PhantomData,
        }
    }

    #[instrument(skip(self), level = Level::INFO)]
    pub async fn run(mut self) {
        loop {
            match self.reset_stuck().await {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        "[RdbPromHandler::run] reset stuck failed",
                    );
                }
            }

            match self.poll().await {
                Ok(rows) => {
                    for row in &rows {
                        self.process_row(row).await;
                    }
                }
                Err(e) => {
                    tracing::error!(
                        error = ?e,
                        "[RdbPromHandler::run] poll failed",
                    );
                }
            }

            tokio::select! {
                _ = sleep(POLL_INTERVAL) => {}
                _ = &mut self.shutdown_recv => {
                    self.accepting.store(false, Ordering::Release);
                    break;
                }
            }
        }

        // Drain one final poll cycle before exiting.
        match self.poll().await {
            Ok(rows) => {
                for row in &rows {
                    self.process_row(row).await;
                }
            }
            Err(e) => {
                tracing::error!(
                    error = ?e,
                    "[RdbPromHandler::run] final poll after shutdown failed",
                );
            }
        }

        self.done_send.send(()).unwrap_or_else(|error| {
            tracing::warn!(
                error = ?error,
                "[RdbPromHandler::run] completion receiver already dropped",
            );
        });
    }

    async fn poll(&self) -> RegularResult<Vec<LocalMessageRow>> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        self.local_message_repo
            .advance(&mut context, &PollPending)
            .await
    }

    async fn process_row(&self, row: &LocalMessageRow) {
        match self.reset_stuck().await {
            Ok(()) => {}
            Err(e) => {
                tracing::error!(
                    id = %row.f_id,
                    error = ?e,
                    "[RdbPromHandler] reset stuck before claim failed",
                );
            }
        }

        let claimed = match self.claim(&row.f_id).await {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(
                    id = %row.f_id,
                    error = ?e,
                    "[RdbPromHandler] claim failed",
                );
                return;
            }
        };

        if !claimed {
            return;
        }

        match dispatch_topic(
            &self.drive,
            &self.repo,
            &self.local_message_repo,
            &self.image_pool,
            &row.f_topic,
            &row.f_payload,
        )
        .await
        {
            TaskOutcome::Complete => match self.complete(&row.f_id).await {
                Ok(()) => {}
                Err(e) => {
                    tracing::error!(
                        id = %row.f_id,
                        error = ?e,
                        "[RdbPromHandler] complete failed",
                    );
                }
            },
            TaskOutcome::Retry(error) => {
                match self.retry(&row.f_id, &error).await {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!(
                            id = %row.f_id,
                            original_error = %error,
                            error = ?e,
                            "[RdbPromHandler] retry mark failed",
                        );
                    }
                }
            }
            TaskOutcome::Dead(error) => {
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    error = %error,
                    "[RdbPromHandler] task failed",
                );

                match self.fail(&row.f_id, &error).await {
                    Ok(()) => {}
                    Err(e) => {
                        tracing::error!(
                            id = %row.f_id,
                            original_error = %error,
                            error = ?e,
                            "[RdbPromHandler] fail mark failed",
                        );
                    }
                }
            }
        }
    }

    async fn claim(&self, id: &str) -> RegularResult<bool> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        self.local_message_repo
            .advance(&mut context, &ClaimStep { id })
            .await
    }

    async fn complete(&self, id: &str) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        self.local_message_repo
            .advance(&mut context, &CompleteStep { id })
            .await
    }

    async fn fail(&self, id: &str, error: &str) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        self.local_message_repo
            .advance(&mut context, &FailStep { id, error })
            .await
    }

    async fn retry(&self, id: &str, error: &str) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);
        let visible_at = OffsetDateTime::now_utc() + RETRY_DELAY;

        self.local_message_repo
            .advance(
                &mut context,
                &RetryStep {
                    id,
                    error,
                    visible_at: &visible_at,
                },
            )
            .await
    }

    async fn reset_stuck(&self) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);
        let before = OffsetDateTime::now_utc() - PROCESSING_TIMEOUT;

        self.local_message_repo
            .advance(&mut context, &ResetStuckStep { before: &before })
            .await
    }
}

/// Route a prom record by topic to the appropriate handler module.
async fn dispatch_topic<D, R, I>(
    drive: &D,
    repo: &Arc<R>,
    local_message_repo: &LocalMessageRepo,
    image_pool: &I,
    topic: &str,
    payload_json: &serde_json::Value,
) -> TaskOutcome
where
    D: Drive<RdbContext>,
    D::Error: Into<RegularError>,
    R: DeriveTransactional
        + ComicRepo<RdbContext>
        + WorksetRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + PageRepo<RdbContext>
        + AssignmentRepo<RdbContext>
        + AssignmentInvitationRepo<RdbContext>
        + UnitRepo<RdbContext>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        ComicRepoTransactional<RdbContext>
            + WorksetRepoTransactional<RdbContext>
            + ChapterRepoTransactional<RdbContext>
            + PageRepoTransactional<RdbContext>
            + TeamRepoTransactional<RdbContext>
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + UserRepoTransactional<RdbContext>
            + Send
            + Sync,
    I: ImagePool + Send + Sync,
{
    match topic {
        IMAGE_TOPIC => {
            let payload_str =
                serde_json::to_string(payload_json).map_err(|e| {
                    format!("failed to serialize image payload: {}", e)
                });

            let payload_str = match payload_str {
                Ok(payload_str) => payload_str,
                Err(e) => return TaskOutcome::Dead(e),
            };

            let task: Result<ImageTask<'_>, String> =
                serde_json::from_str(&payload_str).map_err(|e| {
                    format!("failed to deserialize image task: {}", e)
                });

            let task = match task {
                Ok(task) => task,
                Err(e) => return TaskOutcome::Dead(e),
            };

            image::handle(
                drive,
                repo,
                local_message_repo,
                image_pool,
                &task,
            )
            .await
        }
        unknown => {
            TaskOutcome::Dead(format!("unknown prom topic: {}", unknown))
        }
    }
}
