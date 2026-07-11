//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using the Advance pattern for local-message lifecycle.
//!
//! Topic dispatch routes to [`image`].

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
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
use crate::part::prom::Payload;
use crate::part::prom::task::IMAGE_TOPIC;
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
    ClaimStep, CompleteStep, FailStep, PollPending, RdbPromRepo,
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

pub enum TaskFlow {
    Complete,
    Retry(String),
    Dead(String),
}

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromHandler<D, R, I> {
    core: RdbCore,
    drive: D,

    repo: RdbPromRepo<R>,

    image_pool: I,

    shutdown_recv: OneshotReceiver<()>,
    done_send: OneshotSender<()>,
    accepting: Arc<AtomicBool>,
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
        repo: RdbPromRepo<R>,
        image_pool: I,
        shutdown_recv: OneshotReceiver<()>,
        done_send: OneshotSender<()>,
        accepting: Arc<AtomicBool>,
    ) -> Self {
        Self {
            core,
            drive,
            repo,
            image_pool,
            shutdown_recv,
            done_send,
            accepting,
        }
    }

    #[instrument(skip(self), level = Level::INFO)]
    pub async fn run(mut self) {
        //
        loop {
            //
            if let Err(e) = self.reset_stuck().await {
                tracing::error!(
                    error = ?e,
                    "[RdbPromHandler::run] reset stuck failed",
                );
            }

            match self.poll().await {
                //
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
            //
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
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo.advance(&mut context, &PollPending).await
    }

    async fn process_row(&self, row: &LocalMessageRow) {
        //
        if let Err(e) = self.reset_stuck().await {
            tracing::error!(
                id = %row.f_id,
                error = ?e,
                "[RdbPromHandler] reset stuck before claim failed",
            );
        }

        let claimed = match self.claim(&row.f_id).await {
            //
            Ok(v) => v,

            Err(e) => {
                //
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
            &self.image_pool,
            &row.f_topic,
            &row.f_payload,
        )
        .await
        {
            TaskFlow::Complete => {
                if let Err(e) = self.complete(&row.f_id).await {
                    tracing::error!(
                        id = %row.f_id,
                        error = ?e,
                        "[RdbPromHandler] complete failed",
                    );
                }
            }

            TaskFlow::Retry(error) => {
                if let Err(e) = self.retry(&row.f_id, &error).await {
                    tracing::error!(
                        id = %row.f_id,
                        original_error = %error,
                        error = ?e,
                        "[RdbPromHandler] retry mark failed",
                    );
                }
            }

            TaskFlow::Dead(error) => {
                //
                tracing::error!(
                    id = %row.f_id,
                    topic = %row.f_topic,
                    error = %error,
                    "[RdbPromHandler] task failed",
                );

                if let Err(e) = self.fail(&row.f_id, &error).await {
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

    async fn claim(&self, id: &str) -> RegularResult<bool> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let claim_step = ClaimStep::new(id);

        self.repo.advance(&mut context, &claim_step).await
    }

    async fn complete(&self, id: &str) -> RegularResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let complete_step = CompleteStep::new(id);

        self.repo.advance(&mut context, &complete_step).await
    }

    async fn fail(&self, id: &str, error: &str) -> RegularResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let fail_step = FailStep::new(id, error);

        self.repo.advance(&mut context, &fail_step).await
    }

    async fn retry(&self, id: &str, error: &str) -> RegularResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let visible_at = OffsetDateTime::now_utc() + RETRY_DELAY;

        let retry_step = RetryStep::new(id, error, &visible_at);

        self.repo.advance(&mut context, &retry_step).await
    }

    async fn reset_stuck(&self) -> RegularResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let before = OffsetDateTime::now_utc() - PROCESSING_TIMEOUT;

        let reset_stuck_step = ResetStuckStep::new(&before);

        self.repo.advance(&mut context, &reset_stuck_step).await
    }
}

/// Route a prom record by topic to the appropriate handler module.
async fn dispatch_topic<D, R, I>(
    drive: &D,
    repo: &RdbPromRepo<R>,
    image_pool: &I,
    topic: &str,
    payload: &serde_json::Value,
) -> TaskFlow
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
        //
        IMAGE_TOPIC => {
            //
            let payload_str = serde_json::to_string(payload).map_err(|e| {
                format!("failed to serialize image payload: {}", e)
            });

            let payload_str = match payload_str {
                //
                Ok(payload_str) => payload_str,

                Err(e) => return TaskFlow::Dead(e),
            };

            let payload: Result<Payload<'_>, String> =
                serde_json::from_str(&payload_str).map_err(|e| {
                    format!("failed to deserialize image payload: {}", e)
                });

            let payload = match payload {
                //
                Ok(payload) => payload,

                Err(e) => return TaskFlow::Dead(e),
            };

            let Payload::Image(task) = payload;

            image::handle(drive, repo, image_pool, &task).await
        }

        unknown => TaskFlow::Dead(format!("unknown prom topic: {}", unknown)),
    }
}

#[cfg(all(test, feature = "repo"))]
mod tests {
    // image_payloads_from_rdb_dispatch(dispatch_topic)(positive): payloads stored by the RDB append path are decoded and dispatched by their topic.

    use super::*;

    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use time::{Duration, OffsetDateTime};

    use crate::part::prom::Append;
    use crate::part::prom::task::{ImageKind, ImageTask};
    use crate::part_impl::drive::rdb_impl::RdbDrive;
    use crate::part_impl::prom::rdb_impl::entity::LocalMessageEntry;
    use crate::part_impl::repo::mock_impl::Mock;
    use crate::part_impl::repo::rdb_impl::{RdbRepo, schema, test_shared};

    const PREFIX: &str = "rdb-test-prom-handler-";

    #[tokio::test]
    async fn image_payloads_from_rdb_dispatch() {
        //
        let shared = test_shared::shared().await;

        test_shared::reset(&shared, PREFIX).await;

        let visible_at = OffsetDateTime::now_utc() + Duration::hours(1);

        let delete_image_prom_append = Append {
            id: "rdb-test-prom-handler-delete",
            topic: IMAGE_TOPIC,
            payload: Payload::Image(ImageTask::Delete {
                object_key: "old-avatar.png",
            }),
            visible_at: &visible_at,
        };

        let delete_local_message_entry = LocalMessageEntry::from_append(
            &delete_image_prom_append,
            OffsetDateTime::now_utc(),
        )
        .ok()
        .unwrap();

        let mut conn = shared.get().await.ok().unwrap();

        diesel::insert_into(schema::t_local_message::table)
            .values(&delete_local_message_entry)
            .execute(&mut conn)
            .await
            .ok()
            .unwrap();

        let delete_payload: serde_json::Value = schema::t_local_message::table
            .filter(
                schema::t_local_message::f_id
                    .eq("rdb-test-prom-handler-delete"),
            )
            .select(schema::t_local_message::f_payload)
            .first(&mut conn)
            .await
            .ok()
            .unwrap();

        let drive = RdbDrive::new(shared.clone());

        let rdb_prom_repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

        let delete_image_pool = Mock::new();

        let delete_task_flow = dispatch_topic(
            &drive,
            &rdb_prom_repo,
            &delete_image_pool,
            IMAGE_TOPIC,
            &delete_payload,
        )
        .await;

        assert!(matches!(delete_task_flow, TaskFlow::Complete));

        assert_eq!(
            delete_image_pool.snapshot().deleted_image_keys,
            vec!["old-avatar.png".to_string()]
        );

        let check_uploaded_image_prom_append = Append {
            id: "rdb-test-prom-handler-check-uploaded",
            topic: IMAGE_TOPIC,
            payload: Payload::Image(ImageTask::CheckUploaded {
                kind: ImageKind::UserAvatar,
                resource_id: "missing-user",
                object_key: "new-avatar.png",
                image_version: 1,
            }),
            visible_at: &visible_at,
        };

        let check_uploaded_local_message_entry =
            LocalMessageEntry::from_append(
                &check_uploaded_image_prom_append,
                OffsetDateTime::now_utc(),
            )
            .ok()
            .unwrap();

        diesel::insert_into(schema::t_local_message::table)
            .values(&check_uploaded_local_message_entry)
            .execute(&mut conn)
            .await
            .ok()
            .unwrap();

        let check_uploaded_payload: serde_json::Value =
            schema::t_local_message::table
                .filter(
                    schema::t_local_message::f_id
                        .eq("rdb-test-prom-handler-check-uploaded"),
                )
                .select(schema::t_local_message::f_payload)
                .first(&mut conn)
                .await
                .ok()
                .unwrap();

        let check_uploaded_image_pool = Mock::new().with_image_head_absent();

        let check_uploaded_task_flow = dispatch_topic(
            &drive,
            &rdb_prom_repo,
            &check_uploaded_image_pool,
            IMAGE_TOPIC,
            &check_uploaded_payload,
        )
        .await;

        assert!(matches!(check_uploaded_task_flow, TaskFlow::Complete));

        test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

        test_shared::assert_no_leftovers(&shared, PREFIX)
            .await
            .ok()
            .unwrap();
    }
}
