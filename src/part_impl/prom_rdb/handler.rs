//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using the Advance pattern for local-message lifecycle.
//!
//! Topic dispatch routes to [`image`] and [`comic`].

use std::marker::PhantomData;
use std::sync::Arc;
use std::time::Duration;

use tokio::time::sleep;
use tracing::{Level, instrument};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::task::{
    COMIC_ARCHIVE_TOPIC, ComicTask, IMAGE_TOPIC, ImageTask,
};
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::unit::{UnitRepo, UnitRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::part_impl::prom_rdb::{
    ClaimStep, CompleteStep, FailStep, LocalMessageRow, PollPending,
};
use crate::part_impl::rdb_core::{RdbContext, RdbCore};
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional;

mod comic;
mod image;

const POLL_INTERVAL: Duration = Duration::from_secs(5);

pub struct RdbPromHandler<D, R, P, I> {
    core: RdbCore,
    drive: D,

    repo: Arc<R>,

    prom: P,
    image_pool: I,

    _p: PhantomData<RdbContext>,
}

impl<D, R, P, I> RdbPromHandler<D, R, P, I>
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
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + Send
            + Sync,
    P: Prom<RdbContext>
        + Advance<PollPending, RdbContext, Error = RegularError>
        + for<'a> Advance<ClaimStep<'a>, RdbContext, Error = RegularError>
        + for<'a> Advance<CompleteStep<'a>, RdbContext, Error = RegularError>
        + for<'a> Advance<FailStep<'a>, RdbContext, Error = RegularError>
        + Send
        + Sync
        + 'static,
    I: ImagePool + Send + Sync + 'static,
{
    pub fn new(
        core: RdbCore,
        drive: D,
        repo: Arc<R>,
        prom: P,
        image_pool: I,
    ) -> Self {
        Self {
            core,
            drive,
            repo,
            prom,
            image_pool,
            _p: PhantomData,
        }
    }

    #[instrument(skip(self), level = Level::INFO)]
    pub async fn run(&self) {
        loop {
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

            sleep(POLL_INTERVAL).await;
        }
    }

    async fn poll(&self) -> RegularResult<Vec<LocalMessageRow>> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        Advance::<PollPending, RdbContext>::advance(
            &self.prom,
            &mut context,
            &PollPending,
        )
        .await
    }

    async fn process_row(&self, row: &LocalMessageRow) {
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

        let result = dispatch_topic(
            &self.drive,
            &self.repo,
            &self.prom,
            &self.image_pool,
            &row.f_topic,
            &row.f_payload,
        )
        .await;

        match result {
            Ok(()) => {
                let _ = self.complete(&row.f_id).await;
            }
            Err(e) => {
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    error = ?e,
                    "[RdbPromHandler] task failed",
                );
                let _ = self.fail(&row.f_id, &format!("{:?}", e)).await;
            }
        }
    }

    async fn claim(&self, id: &str) -> RegularResult<bool> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        Advance::<ClaimStep<'_>, RdbContext>::advance(
            &self.prom,
            &mut context,
            &ClaimStep { id },
        )
        .await
    }

    async fn complete(&self, id: &str) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        Advance::<CompleteStep<'_>, RdbContext>::advance(
            &self.prom,
            &mut context,
            &CompleteStep { id },
        )
        .await
    }

    async fn fail(&self, id: &str, error: &str) -> RegularResult<()> {
        let conn = self.core.get().await?;
        let mut context = RdbContext::new(conn);

        Advance::<FailStep<'_>, RdbContext>::advance(
            &self.prom,
            &mut context,
            &FailStep { id, error },
        )
        .await
    }
}

/// Route a prom record by topic to the appropriate handler module.
async fn dispatch_topic<D, R, P, I>(
    drive: &D,
    repo: &Arc<R>,
    prom: &P,
    image_pool: &I,
    topic: &str,
    payload_json: &serde_json::Value,
) -> RegularResult<()>
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
            + AssignmentRepoTransactional<RdbContext>
            + AssignmentInvitationRepoTransactional<RdbContext>
            + UnitRepoTransactional<RdbContext>
            + Send
            + Sync,
    P: Prom<RdbContext> + Send + Sync,
    I: ImagePool + Send + Sync,
{
    match topic {
        IMAGE_TOPIC => {
            let payload_str =
                serde_json::to_string(payload_json).map_err(|e| {
                    RegularError::Unrecoverable {
                        message: format!(
                            "failed to serialize image payload: {}",
                            e
                        ),
                    }
                })?;

            let task: ImageTask<'_> = serde_json::from_str(&payload_str)
                .map_err(|e| RegularError::Unrecoverable {
                    message: format!("failed to deserialize image task: {}", e),
                })?;

            image::handle(drive, repo, prom, image_pool, &task).await
        }
        COMIC_ARCHIVE_TOPIC => {
            let payload_str =
                serde_json::to_string(payload_json).map_err(|e| {
                    RegularError::Unrecoverable {
                        message: format!(
                            "failed to serialize comic payload: {}",
                            e
                        ),
                    }
                })?;

            let task: ComicTask<'_> = serde_json::from_str(&payload_str)
                .map_err(|e| RegularError::Unrecoverable {
                    message: format!("failed to deserialize comic task: {}", e),
                })?;

            comic::handle(drive, repo, prom, image_pool, &task).await
        }
        unknown => Err(RegularError::Unrecoverable {
            message: format!("unknown prom topic: {}", unknown),
        }),
    }
}
