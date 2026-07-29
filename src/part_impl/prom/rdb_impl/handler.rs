//! Prom-consumer worker that polls, dispatches by topic, and executes
//! deferred actions using coordinated repository operations.
//!
//! Topic dispatch routes to [`image`].

use std::time::Duration as StdDuration;

use poprako_orchestra::{Nucl, Step as _};
use time::{Duration, OffsetDateTime};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::image::ImageManager;
use crate::part::prom::payload::Payload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::entity::LocalMessageRow;
use crate::part_impl::prom::rdb_impl::repo::{
    ClaimPending, CompleteMessage, FailMessage, PollPending, PurgeCompleted,
    RdbPromRepo, ResetStuck, RetryMessage,
};
use crate::part_impl::shared::{RdbContext, RdbCore};
use crate::result::{BaseError, BaseResult};

/// Prom image event handler.
mod image;
/// Prom invitation event handler.
mod invitation;
#[cfg(all(test, feature = "repo"))]
mod tests {
    // image_payloads_from_rdb_dispatch(dispatch_payload)(positive): payloads stored by the RDB defer path are decoded and dispatched by their topic.

    use super::*;

    use diesel::prelude::*;
    use diesel_async::RunQueryDsl;
    use poprako_orchestra_extra::prom::task::Task;
    use time::OffsetDateTime;

    use crate::part::prom::payload::image;
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

        let delete_id = "rdb-test-prom-handler-delete".to_string();

        let delete_payload = Payload::Image(image::Payload::Delete {
            object_key: "old-avatar.png".to_string(),
        });

        let delete_task = Task {
            id: &delete_id,
            payload: &delete_payload,
            delay: None,
        };

        let delete_local_message_entry = LocalMessageEntry::from_task(
            &delete_task,
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

        let nucl = RdbDrive::new(shared.clone());

        let rdb_prom_repo = RdbPromRepo::new(RdbRepo::new(shared.clone()));

        let image_pool = Mock::new();

        let delete_task_flow = dispatch_payload(
            &nucl,
            rdb_prom_repo.inner(),
            &image_pool,
            "image",
            &delete_payload,
        )
        .await;

        assert!(matches!(delete_task_flow, TaskFlow::Complete));

        assert_eq!(
            image_pool.snapshot().deleted_image_keys,
            vec!["old-avatar.png".to_string()]
        );

        let check_id = "rdb-test-prom-handler-check-uploaded".to_string();

        let check_payload = Payload::Image(image::Payload::CheckUpload {
            resource_kind: image::ResourceKind::UserAvatar,
            resource_id: "missing-user".to_string(),
            object_key: "new-avatar.png".to_string(),
            version: 1,
        });

        let check_task = Task {
            id: &check_id,
            payload: &check_payload,
            delay: None,
        };

        let check_uploaded_local_message_entry = LocalMessageEntry::from_task(
            &check_task,
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

        let image_pool = Mock::new().with_image_head_absent();

        let check_uploaded_task_flow = dispatch_payload(
            &nucl,
            rdb_prom_repo.inner(),
            &image_pool,
            "image",
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

/// Interval between successive poll cycles in the prom background worker.
const POLL_INTERVAL: StdDuration = StdDuration::from_secs(300);
const RETRY_DELAY: Duration = Duration::minutes(5);
const PROCESSING_TIMEOUT: Duration = Duration::minutes(15);
const COMPLETED_RETENTION: Duration = Duration::days(7);
const COMPLETED_PURGE_INTERVAL: Duration = Duration::hours(1);

/// Outcome of processing one prom task — dictates how the handler updates the record.
pub enum TaskFlow {
    /// Task completed successfully; move record to Completed status.
    Complete,
    /// Task encountered a transient error; schedule for retry.
    Retry(String),
    /// Task encountered a fatal error; move record to Dead status.
    Dead(String),
}

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromHandler<D, R, I> {
    core: RdbCore,
    nucl: D,

    repo: RdbPromRepo<R>,

    image_pool: I,

    token: CancellationToken,
}

impl<D, R, I> RdbPromHandler<D, R, I>
where
    D: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ComicRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + TeamRepo<RdbContext>
        + UserRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    I: ImageManager + Send + Sync + 'static,
{
    /// Builds a new prom background handler from its core, nucl, repo, and lifecycle channels.
    pub fn new(
        core: RdbCore,
        nucl: D,
        repo: RdbPromRepo<R>,
        image_pool: I,
        token: CancellationToken,
    ) -> Self {
        Self {
            core,
            nucl,
            repo,
            image_pool,
            token,
        }
    }

    #[instrument(level = "info", skip_all)]
    /// Runs the prom polling loop until cancellation, finishing the current cycle before exit.
    pub async fn run(self) {
        //
        let mut next_completed_purge_at = OffsetDateTime::now_utc();

        loop {
            //
            if self.token.is_cancelled() {
                break;
            }

            //
            let now = OffsetDateTime::now_utc();

            if now >= next_completed_purge_at {
                //
                match self.purge_completed().await {
                    //
                    Ok(purged_count) => {
                        if purged_count > 0 {
                            tracing::info!(
                                purged_count,
                                "[RdbPromHandler::run] purged expired completed messages",
                            );
                        }
                    }

                    Err(error) => {
                        tracing::error!(
                            error = ?error,
                            "[RdbPromHandler::run] purge completed failed",
                        );
                    }
                }

                next_completed_purge_at = now + COMPLETED_PURGE_INTERVAL;
            }

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
                biased;
                () = self.token.cancelled() => break,
                _ = sleep(POLL_INTERVAL) => {}
            }
        }
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn poll(&self) -> BaseResult<Vec<LocalMessageRow>> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo.step(&mut context, &PollPending).await
    }

    #[instrument(level = "info", skip_all)]
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

        match dispatch_payload(
            &self.nucl,
            self.repo.inner(),
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

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn claim(&self, id: &str) -> BaseResult<bool> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo.step(&mut context, &ClaimPending::new(id)).await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn complete(&self, id: &str) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo
            .step(&mut context, &CompleteMessage::new(id))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn fail(&self, id: &str, error: &str) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        self.repo
            .step(&mut context, &FailMessage::new(id, error))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn retry(&self, id: &str, error: &str) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let visible_at = OffsetDateTime::now_utc() + RETRY_DELAY;

        self.repo
            .step(&mut context, &RetryMessage::new(id, error, &visible_at))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn reset_stuck(&self) -> BaseResult<()> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let before = OffsetDateTime::now_utc() - PROCESSING_TIMEOUT;

        self.repo
            .step(&mut context, &ResetStuck::new(&before))
            .await
    }

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn purge_completed(&self) -> BaseResult<usize> {
        //
        let conn = self.core.get().await?;

        let mut context = RdbContext::new(conn);

        let before = OffsetDateTime::now_utc() - COMPLETED_RETENTION;

        self.repo
            .step(&mut context, &PurgeCompleted::new(&before))
            .await
    }
}

/// Decodes and dispatches one persisted prom payload.
#[instrument(level = "info", skip_all)]
async fn dispatch_payload<D, R, I>(
    nucl: &D,
    repo: &R,
    image_pool: &I,
    topic: &str,
    payload: &serde_json::Value,
) -> TaskFlow
where
    D: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ComicRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + TeamRepo<RdbContext>
        + UserRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
{
    let payload: Payload = match serde_json::from_value(payload.clone()) {
        //
        Ok(payload) => payload,

        Err(error) => {
            return TaskFlow::Dead(format!(
                "failed to deserialize prom payload: {}",
                error
            ));
        }
    };

    if payload.topic() != topic {
        return TaskFlow::Dead(format!(
            "prom topic {} does not match payload topic {}",
            topic,
            payload.topic()
        ));
    }

    match payload {
        //
        Payload::Image(task) => {
            image::handle(nucl, repo, image_pool, &task).await
        }

        Payload::PurgeExpiredInvitation(event) => {
            invitation::handle(nucl, repo, &event).await
        }
    }
}
