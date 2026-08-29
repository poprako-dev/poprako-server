//! Shared types and dispatch logic for the prom actor submodules.
//!
//! Defined here so that both the parent [`actor`] and its child modules
//! (notably [`pool`]) can import without creating an upward ancestor
//! dependency.
//!
//! [`actor`]: crate::part_impl::prom::rdb_impl::actor
//! [`pool`]: crate::part_impl::prom::rdb_impl::actor::pool

use poprako_orchestra::Nucl;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_obj_dept::ObjDept;

use crate::part::effect::Develop;
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::actor::task_flow::TaskFlow;
use crate::part_impl::prom::rdb_impl::actor::{chapter, invitation};
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::result::BaseError;
use crate::shared::RdbContext;

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromActor<N, R, O, D> {
    /// Transaction coordinator used for actor-level database operations.
    nucl: N,

    /// Repository wrapping message lifecycle and domain queries.
    repo: RdbPromRepo<R>,

    /// Total object department used by business workflow checks.
    obj_dept: O,

    /// Shared side-effect developer for automatic workflow events.
    develop: D,

    /// Shutdown signal propagated from the owning [`RdbProm`].
    token: CancellationToken,
}

impl<N, R, O, D> RdbPromActor<N, R, O, D>
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
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
    D: Develop + Send + Sync + 'static,
{
    /// Builds a new prom background actor from its core, nucl, repo, and lifecycle channels.
    pub const fn new(
        nucl: N,
        repo: RdbPromRepo<R>,
        obj_dept: O,
        develop: D,
        token: CancellationToken,
    ) -> Self {
        //
        Self {
            nucl,
            repo,
            obj_dept,
            develop,
            token,
        }
    }

    /// Returns the transaction coordinator used by the actor.
    #[must_use]
    pub const fn nucl(&self) -> &N {
        &self.nucl
    }

    /// Returns the repository used for message lifecycle and domain queries.
    #[must_use]
    pub const fn repo(&self) -> &RdbPromRepo<R> {
        &self.repo
    }

    /// Returns the total object department used by workflow checks.
    #[must_use]
    pub const fn obj_dept(&self) -> &O {
        &self.obj_dept
    }

    /// Returns the side-effect developer used for workflow events.
    #[must_use]
    pub const fn develop(&self) -> &D {
        &self.develop
    }

    /// Returns the cancellation token that controls the actor lifecycle.
    #[must_use]
    pub const fn token(&self) -> &CancellationToken {
        &self.token
    }
}

/// Decodes and dispatches one persisted prom payload.
#[instrument(level = "info", skip_all)]
pub async fn dispatch_payload<N, R, O, D>(
    nucl: &N,
    repo: &R,
    obj_dept: &O,
    develop: &D,
    topic: &str,
    payload: &serde_json::Value,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
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
    O: ObjDept<PageImage, RdbContext> + Sync,
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
                    error,
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
            chapter::handle(nucl, repo, obj_dept, develop, &task).await
        }

        TaskPayload::Invitation { payload: event } => {
            invitation::handle(repo, &event).await
        }
    }
}
