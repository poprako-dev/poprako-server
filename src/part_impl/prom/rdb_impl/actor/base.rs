//! Shared types and task dispatch logic for the prom actor submodules.
//!
//! Defined here so that both the parent [`actor`] and its child modules
//! (notably [`pool`]) can import without creating an upward ancestor
//! dependency.
//!
//! [`actor`]: crate::part_impl::prom::rdb_impl::actor
//! [`pool`]: crate::part_impl::prom::rdb_impl::actor::pool

use poprako_orchestra::Step;
use tokio_util::sync::CancellationToken;
use tracing::instrument;

use poprako_obj_dept::oper::GetObjMeta;
use poprako_obj_dept::pool::ObjPoolView;
use poprako_obj_dept::rest::ObjDeptError;
use poprako_rdb_core::RdbCore;

use crate::part::effect::Develop;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::PageImage;
use crate::part::prom::payload::TaskPayload;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::prom::rdb_impl::repo::RdbPromRepo;
use crate::part_impl::prom::task_flow::TaskFlow;
use crate::part_impl::repo::HybRepo;
use crate::shared::RdbContext;
use crate::usecase::prom::{PromTaskAction, handle_chapter, handle_invitation};

/// Read-only object capabilities available to the general prom actor.
pub trait ObjView:
    ObjPoolView
    + for<'a> Step<
        GetObjMeta<'a, PageImage>,
        RdbContext,
        Level = ReptRead,
        Error = ObjDeptError,
    >
{
}

impl<T> ObjView for T where
    T: ObjPoolView
        + for<'a> Step<
            GetObjMeta<'a, PageImage>,
            RdbContext,
            Level = ReptRead,
            Error = ObjDeptError,
        >
{
}

/// Background worker that polls the `t_local_message` table, dispatches by topic,
/// and completes or fails each record.
pub struct RdbPromActor<V, D> {
    //
    /// Shared relational database core.
    core: RdbCore,

    /// Repository implementing persisted message lifecycle operations.
    repo: RdbPromRepo,

    /// Read-only object capabilities used by deferred checks.
    obj_view: V,

    /// Effect dispatcher used after committed business changes.
    develop: D,

    /// Shutdown signal propagated from the owning [`RdbProm`].
    token: CancellationToken,
}

impl<V, D> RdbPromActor<V, D> {
    /// Builds a prom actor from its core and read-only object view.
    pub const fn new(
        core: RdbCore,
        obj_view: V,
        develop: D,
        token: CancellationToken,
    ) -> Self {
        //
        Self {
            core,
            repo: RdbPromRepo::new(),
            obj_view,
            develop,
            token,
        }
    }

    /// Returns a transaction coordinator over the shared core.
    #[must_use]
    pub fn nucl(&self) -> RdbNucl {
        RdbNucl::new(self.core.clone())
    }

    /// Returns the repository used for persisted message lifecycle operations.
    #[must_use]
    pub const fn repo(&self) -> &RdbPromRepo {
        &self.repo
    }

    /// Returns a business repository over the shared core.
    #[must_use]
    pub fn task_repo(&self) -> HybRepo {
        HybRepo::new(self.core.clone())
    }

    /// Returns the object department used by deferred object checks.
    #[must_use]
    pub const fn obj_view(&self) -> &V {
        &self.obj_view
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

impl<V, D> RdbPromActor<V, D>
where
    V: ObjView + Send + Sync,
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

        let nucl = self.nucl();

        let task_repo = self.task_repo();

        let action = match task {
            //
            TaskPayload::Chapter { payload } => {
                //
                handle_chapter(
                    (&nucl, &task_repo, self.obj_view(), self.develop()),
                    &payload,
                )
                .await
            }

            TaskPayload::Invitation { payload } => {
                handle_invitation((&task_repo,), &payload).await
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
