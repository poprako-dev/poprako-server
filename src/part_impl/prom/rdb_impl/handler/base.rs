//! Shared types and dispatch logic for the prom handler submodules.
//!
//! Defined here so that both the parent [`handler`] and its child modules
//! (notably [`pool`]) can import without creating an upward ancestor
//! dependency.
//!
//! [`handler`]: crate::part_impl::prom::rdb_impl::handler
//! [`pool`]: crate::part_impl::prom::rdb_impl::handler::pool

use poprako_orchestra::Nucl;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use crate::part::effect::Develop;
use crate::part::image::ImageManager;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::prom::rdb_impl::handler::{chapter, image, invitation};
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::result::BaseError;
use crate::shared::RdbContext;

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromHandler<N, R, I, D> {
    //
    /// Transaction coordinator used for handler-level database operations.
    pub nucl: N,

    /// Repository wrapping message lifecycle and domain queries.
    pub repo: RdbPromRepo<R>,

    /// Object storage client for image verification and cleanup.
    pub image_pool: I,
    /// Shared side-effect developer for automatic workflow events.
    pub develop: D,

    /// Shutdown signal propagated from the owning [`RdbProm`].
    pub token: CancellationToken,
}

impl<N, R, I, D> RdbPromHandler<N, R, I, D>
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + ComicRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + TeamRepo<RdbContext>
        + UserRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    I: ImageManager + Send + Sync + 'static,
    D: Develop + Send + Sync + 'static,
{
    /// Builds a new prom background handler from its core, nucl, repo, and lifecycle channels.
    pub fn new(
        nucl: N,
        repo: RdbPromRepo<R>,
        image_pool: I,
        develop: D,
        token: CancellationToken,
    ) -> Self {
        //
        Self {
            nucl,
            repo,
            image_pool,
            develop,
            token,
        }
    }
}

/// Decodes and dispatches one persisted prom payload.
#[instrument(level = "info", skip_all)]
pub async fn dispatch_payload<N, R, I, D>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    develop: &D,
    topic: &str,
    payload: &serde_json::Value,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + ChapterWorkflowRecordRepo<RdbContext>
        + ComicRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + TeamRepo<RdbContext>
        + UserRepo<RdbContext>
        + Send
        + Sync,
    I: ImageManager + Send + Sync,
    D: Develop + Sync,
{
    let payload = match serde_json::from_value::<TaskPayload>(payload.clone()) {
        //
        Ok(payload) => payload,

        Err(error) => {
            //
            tracing::error!(
                operation = "deserialize_prom_payload",
                sdk_err = ?error,
                "JSON SDK deserialization error",
            );

            return TaskFlow::Dead {
                err_message: format!(
                    "failed to deserialize prom payload: {}",
                    error
                ),
            };
        }
    };

    if payload.topic() != topic {
        //
        return TaskFlow::Dead {
            err_message: format!(
                "prom topic {} does not match payload topic {}",
                topic,
                payload.topic()
            ),
        };
    }

    match payload {
        //
        TaskPayload::Chapter { payload: task } => {
            chapter::handle(nucl, repo, develop, &task).await
        }

        TaskPayload::Image { payload: task } => {
            image::handle(nucl, repo, image_pool, &task).await
        }

        TaskPayload::Invitation { payload: event } => {
            invitation::handle(repo, &event).await
        }
    }
}
