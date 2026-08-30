//! Shared types and task dispatch logic for the prom actor submodules.
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
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::part_impl::repo::HybRepo;
use crate::result::BaseError;
use crate::shared::RdbContext;
use crate::usecase::prom::{PromTaskAction, handle_chapter, handle_invitation};

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromActor<N, O, D> {
    /// Transaction coordinator used for actor-level database operations.
    nucl: N,

    /// Repository implementing persisted message lifecycle operations.
    repo: RdbPromRepo,

    /// Repository used by deferred business tasks.
    task_repo: HybRepo,

    /// Object department used by deferred object checks.
    obj_dept: O,

    /// Effect dispatcher used after committed business changes.
    develop: D,

    /// Shutdown signal propagated from the owning [`RdbProm`].
    token: CancellationToken,
}

impl<N, O, D> RdbPromActor<N, O, D> {
    /// Builds a prom actor with its queue and business dependencies.
    pub const fn new(
        nucl: N,
        repo: RdbPromRepo,
        task_repo: HybRepo,
        obj_dept: O,
        develop: D,
        token: CancellationToken,
    ) -> Self {
        //
        Self {
            nucl,
            repo,
            task_repo,
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

    /// Returns the repository used for persisted message lifecycle operations.
    #[must_use]
    pub const fn repo(&self) -> &RdbPromRepo {
        &self.repo
    }

    /// Returns the repository used by deferred business tasks.
    #[must_use]
    pub const fn task_repo(&self) -> &HybRepo {
        &self.task_repo
    }

    /// Returns the object department used by deferred object checks.
    #[must_use]
    pub const fn obj_dept(&self) -> &O {
        &self.obj_dept
    }

    /// Returns the effect dispatcher used after committed business changes.
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

impl<N, O, D> RdbPromActor<N, O, D>
where
    N: Nucl<Context = RdbContext, Error = BaseError> + Sync,
    O: ObjDept<PageImage, RdbContext> + Send + Sync,
    D: Develop + Send + Sync,
{
    /// Decodes and dispatches one persisted prom payload.
    #[instrument(level = "info", skip_all)]
    pub async fn dispatch_payload(
        &self,
        topic: &str,
        payload: &serde_json::Value,
    ) -> TaskFlow {
        //
        let task = match serde_json::from_value::<TaskPayload>(payload.clone())
        {
            //
            Ok(task) => task,

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

        if task.topic() != topic {
            //
            return TaskFlow::Dead {
                err_message: format!(
                    "prom topic {} does not match payload topic {}",
                    topic,
                    task.topic()
                ),
            };
        }

        let action = match task {
            //
            TaskPayload::Chapter { payload } => {
                //
                handle_chapter(
                    (
                        self.nucl(),
                        self.task_repo(),
                        self.obj_dept(),
                        self.develop(),
                    ),
                    &payload,
                )
                .await
            }

            TaskPayload::Invitation { payload } => {
                handle_invitation((self.task_repo(),), &payload).await
            }
        };

        match action {
            //
            PromTaskAction::Complete => TaskFlow::Complete,

            PromTaskAction::Retry { message } => TaskFlow::Retry {
                err_message: message,
            },

            PromTaskAction::Wait { message } => TaskFlow::Wait {
                err_message: message,
            },
        }
    }
}
