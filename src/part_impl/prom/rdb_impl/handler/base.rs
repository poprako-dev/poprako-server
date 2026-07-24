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

use crate::part::effect::EffectDevelop;
use crate::part::image::ImageManager;
use crate::part::prom::payload::Payload;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::part_impl::prom::rdb_impl::handler::task_flow::TaskFlow;
use crate::part_impl::prom::rdb_impl::handler::{chapter, image, invitation};
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::shared::{RdbContext, RdbCore};
use crate::result::BaseError;

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromHandler<N, R, I, V> {
    /// Database connection pool for handler-internal queries.
    pub(super) core: RdbCore,
    /// Transaction coordinator used for handler-level database operations.
    pub(super) nucl: N,

    /// Repository wrapping message lifecycle and domain queries.
    pub(super) repo: RdbPromRepo<R>,

    /// Object storage client for image verification and cleanup.
    pub(super) image_pool: I,
    /// Shared side-effect developer for automatic workflow events.
    pub(super) develop: V,

    /// Shutdown signal propagated from the owning [`RdbProm`].
    pub(super) token: CancellationToken,
}

impl<N, R, I, V> RdbPromHandler<N, R, I, V>
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
        + ComicRepo<RdbContext>
        + MemberInvitationRepo<RdbContext>
        + PageRepo<RdbContext>
        + TeamRepo<RdbContext>
        + UserRepo<RdbContext>
        + Send
        + Sync
        + 'static,
    I: ImageManager + Send + Sync + 'static,
    V: EffectDevelop + Send + Sync + 'static,
{
    /// Builds a new prom background handler from its core, nucl, repo, and lifecycle channels.
    pub fn new(
        core: RdbCore,
        nucl: N,
        repo: RdbPromRepo<R>,
        image_pool: I,
        develop: V,
        token: CancellationToken,
    ) -> Self {
        Self {
            core,
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
pub async fn dispatch_payload<N, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    develop: &(impl EffectDevelop + Sync),
    topic: &str,
    payload: &serde_json::Value,
) -> TaskFlow
where
    N: Nucl<Context = RdbContext, Error = BaseError>,
    R: AssignmentInvitationRepo<RdbContext>
        + ChapterRepo<RdbContext>
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
        Payload::AdvanceRawProvide(task) => {
            chapter::handle(nucl, repo, develop, &task).await
        }

        Payload::Image(task) => {
            image::handle(nucl, repo, image_pool, &task).await
        }

        Payload::PurgeExpiredInvitation(event) => {
            invitation::handle(repo, &event).await
        }
    }
}
