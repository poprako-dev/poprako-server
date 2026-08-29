//! Single object-task actor and mechanical state classification.

use std::cmp::Ordering;
use std::future::Future;

use tokio::sync::watch;
use tokio_util::sync::CancellationToken;

use crate::model::task::{ObjPromTask, ObjTaskAction, validate_task};
use crate::prom::ObjProm;
use crate::rdb_impl::ObjRdbRow;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Delay between idle polls.
pub const POLL_INTERVAL: std::time::Duration =
    std::time::Duration::from_secs(5);

/// Maximum duration of one remote request.
pub const REMOTE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(45);

/// Maximum duration of one claimed attempt.
pub const ATTEMPT_TIMEOUT: std::time::Duration =
    std::time::Duration::from_mins(1);

/// Relative position of a task key against the latest object row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjKeyState {
    /// No row remains.
    Missing,

    /// The task precedes the current version.
    Stale,

    /// The current row retains only its watermark.
    Retired,

    /// The current version awaits remote verification.
    Pending,

    /// The current version is verified.
    Verified,

    /// The task is newer than the persisted watermark.
    Future,
}

/// Classifies a task version against the latest object row.
///
/// # Errors
///
/// Returns an error when persisted object state is invalid.
pub fn classify(
    version: u32,
    row: Option<&ObjRdbRow>,
) -> ObjDeptRest<ObjKeyState> {
    //
    let Some(row) = row else {
        return Ok(ObjKeyState::Missing);
    };

    let watermark = u32::try_from(row.version).map_err(|_| {
        //
        ObjDeptError::Unrecoverable {
            message: "object version is outside u32".into(),
        }
    })?;

    match version.cmp(&watermark) {
        //
        Ordering::Less => Ok(ObjKeyState::Stale),

        Ordering::Greater => Ok(ObjKeyState::Future),

        Ordering::Equal => match (&row.f_is_uploaded, &row.hash, &row.ext) {
            //
            (None, None, None) => Ok(ObjKeyState::Retired),

            (Some(false), Some(_), Some(_)) => Ok(ObjKeyState::Pending),

            (Some(true), Some(_), Some(_)) => Ok(ObjKeyState::Verified),

            _ => Err(ObjDeptError::Unrecoverable {
                message: "invalid object row".into(),
            }),
        },
    }
}

/// Control descriptor for the single object actor.
#[derive(Clone)]
pub struct ObjActorDesc {
    /// Actor cancellation signal.
    token: CancellationToken,
    /// Actor completion receiver.
    done_recv: watch::Receiver<bool>,
}

impl ObjActorDesc {
    /// Signals cancellation.
    pub fn cancel(&self) {
        self.token.cancel();
    }

    /// Waits for actor completion.
    pub async fn join(&self) {
        //
        let mut done_recv = self.done_recv.clone();

        if let Err(err) = done_recv.wait_for(|f_is_done| *f_is_done).await {
            //
            tracing::error!(
                operation = "join_obj_actor",
                sdk_err = ?err,
                "ObjDept actor completion channel failed",
            );
        }
    }
}

/// Namespace for constructing the single object actor.
pub struct ObjActor;

impl ObjActor {
    /// Spawns the object actor and returns its descriptor.
    #[expect(
        clippy::new_ret_no_self,
        reason = "ObjActor is the event-loop constructor namespace required by the actor contract"
    )]
    pub fn new<P, H, F>(prom: P, handler: H) -> ObjActorDesc
    where
        P: ObjProm,
        H: Fn(ObjPromTask) -> F + Send + Sync + 'static,
        F: Future<Output = ObjDeptRest<ObjTaskAction>> + Send + 'static,
    {
        let token = CancellationToken::new();

        let actor_token = token.clone();

        let (done_send, done_recv) = watch::channel(false);

        tokio::spawn(async move {
            //
            run_actor(prom, handler, actor_token).await;

            done_send.send_replace(true);
        });

        ObjActorDesc { token, done_recv }
    }
}

// Maps one adapter failure to its mechanical durable-task action.
fn action_from_err(err: ObjDeptError) -> ObjTaskAction {
    //
    match err {
        //
        ObjDeptError::Retryable { message }
        | ObjDeptError::Conflict { message } => {
            ObjTaskAction::Retry { message }
        }

        ObjDeptError::Invalid { message }
        | ObjDeptError::Unrecoverable { message } => {
            ObjTaskAction::Operator { message }
        }
    }
}

// Persists one actor decision through the durable-task adapter.
async fn rest<P>(
    prom: &P,
    task: &ObjPromTask,
    action: &ObjTaskAction,
) -> ObjDeptRest<usize>
where
    P: ObjProm,
{
    //
    match action {
        //
        ObjTaskAction::Complete => prom.complete_task(task).await,

        ObjTaskAction::Retry { message } => {
            prom.retry_task(task, message).await
        }

        ObjTaskAction::Operator { message } => {
            prom.mark_task_operator(task, message).await
        }
    }
}

// Waits until another poll is due or cancellation is requested.
async fn wait_poll(token: &CancellationToken) -> bool {
    //
    tokio::select! {
        () = token.cancelled() => false,
        () = tokio::time::sleep(POLL_INTERVAL) => true,
    }
}

// Runs typed dispatch and persists its fenced task mutation.
async fn run_attempt<P, H, F>(
    prom: &P,
    handler: &H,
    task: &ObjPromTask,
) -> ObjDeptRest<usize>
where
    P: ObjProm,
    H: Fn(ObjPromTask) -> F,
    F: Future<Output = ObjDeptRest<ObjTaskAction>>,
{
    //
    let action = match validate_task(task) {
        //
        Ok(()) => match handler(task.clone()).await {
            //
            Ok(action) => action,

            Err(err) => action_from_err(err),
        },

        Err(err) => action_from_err(err),
    };

    if let ObjTaskAction::Operator { message } = &action {
        //
        tracing::error!(
            task_id = task.id,
            err_message = %message,
            "ObjDept task requires operator repair",
        );
    }

    rest(prom, task, &action).await
}

// Runs the globally serial durable-task loop.
async fn run_actor<P, H, F>(prom: P, handler: H, token: CancellationToken)
where
    P: ObjProm,
    H: Fn(ObjPromTask) -> F,
    F: Future<Output = ObjDeptRest<ObjTaskAction>>,
{
    loop {
        //
        let reset = tokio::select! {
            () = token.cancelled() => break,
            reset = tokio::time::timeout(
                ATTEMPT_TIMEOUT,
                prom.reset_tasks(),
            ) => reset,
        };

        match reset {
            //
            Ok(Ok(_)) => {}

            Ok(Err(err)) => tracing::error!(
                operation = "reset_obj_tasks",
                sdk_err = ?err,
                "ObjDept task reset failed",
            ),

            Err(_) => tracing::error!(
                operation = "reset_obj_tasks",
                "ObjDept task reset timed out",
            ),
        }

        let claimed = tokio::select! {
            () = token.cancelled() => break,
            claimed = tokio::time::timeout(
                ATTEMPT_TIMEOUT,
                prom.claim_task(),
            ) => claimed,
        };

        let task = match claimed {
            //
            Ok(Ok(Some(task))) => task,

            Ok(Ok(None)) => {
                //
                if !wait_poll(&token).await {
                    break;
                }

                continue;
            }

            Ok(Err(err)) => {
                //
                tracing::error!(
                    operation = "claim_obj_task",
                    sdk_err = ?err,
                    "ObjDept task claim failed",
                );

                if !wait_poll(&token).await {
                    break;
                }

                continue;
            }

            Err(_) => {
                //
                tracing::error!(
                    operation = "claim_obj_task",
                    "ObjDept task claim timed out",
                );

                continue;
            }
        };

        let attempt = run_attempt(&prom, &handler, &task);

        let changed = tokio::select! {
            () = token.cancelled() => break,
            changed = tokio::time::timeout(ATTEMPT_TIMEOUT, attempt) => changed,
        };

        match changed {
            //
            Ok(Ok(1)) => {}

            Ok(Ok(0)) => tracing::warn!(
                task_id = task.id,
                lease = task.lease,
                "ObjDept task lost its lease",
            ),

            Ok(Ok(updated_count)) => tracing::error!(
                task_id = task.id,
                updated_count,
                "ObjDept task changed multiple rows",
            ),

            Ok(Err(err)) => tracing::error!(
                task_id = task.id,
                sdk_err = ?err,
                "ObjDept task update failed",
            ),

            Err(_) => tracing::error!(
                task_id = task.id,
                "ObjDept task attempt timed out",
            ),
        }
    }
}

// Runs one typed object handler branch.
/// Runs one typed object handler branch.
#[doc(hidden)]
#[macro_export]
// Expands the typed object handler selected by the manifest dispatch.
macro_rules! __obj_handle {
    ($core:expr, $pool:expr, $task:expr, $obj_mod:ident $(,)?) => {{
        use ::poprako_obj_dept::pool::ObjPool as _;

        let core = $core;
        let pool = $pool;
        let task = $task;
        let key = task.key()?;
        let physical_key = key.encode($obj_mod::NAMESPACE);
        let mut conn = core
            .get()
            .await
            .map_err(::poprako_obj_dept::rdb_impl::rdb_err)?;
        let row = $obj_mod::load(&mut conn, &key.id, false).await?;
        let state =
            ::poprako_obj_dept::actor::classify(key.version, row.as_ref())?;

        drop(conn);

        match (task.oper.as_str(), state) {
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::ObjKeyState::Verified,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete),
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::REMOTE_TIMEOUT,
                    pool.del(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object delete timed out".into(),
                    }
                })??;

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::ObjKeyState::Pending,
            ) => {
                let f_exists = ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::REMOTE_TIMEOUT,
                    pool.has(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object check timed out".into(),
                    }
                })??;
                let mut conn = core
                    .get()
                    .await
                    .map_err(::poprako_obj_dept::rdb_impl::rdb_err)?;
                let updated = match f_exists {
                    true => {
                        $obj_mod::verify(&mut conn, &key.id, key.version)
                            .await?
                    }
                    false => {
                        $obj_mod::retire(&mut conn, &key.id, key.version)
                            .await?
                    }
                };

                let changed_state = match updated {
                    0 => {
                        let row = $obj_mod::load(&mut conn, &key.id, false)
                            .await?;

                        Some(::poprako_obj_dept::actor::classify(
                            key.version,
                            row.as_ref(),
                        )?)
                    }
                    1 => None,
                    _ => {
                        drop(conn);

                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                                message: "object check changed multiple rows".into(),
                            },
                        );
                    }
                };

                drop(conn);

                let f_delete = match (updated, changed_state) {
                    (1, _) => !f_exists,
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::ObjKeyState::Missing
                            | ::poprako_obj_dept::actor::ObjKeyState::Stale
                            | ::poprako_obj_dept::actor::ObjKeyState::Retired,
                        ),
                    ) => true,
                    (0, Some(::poprako_obj_dept::actor::ObjKeyState::Verified)) => {
                        false
                    }
                    (0, Some(::poprako_obj_dept::actor::ObjKeyState::Pending)) => {
                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Retry {
                                message: "object changed during check".into(),
                            },
                        );
                    }
                    (0, Some(::poprako_obj_dept::actor::ObjKeyState::Future)) => {
                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                                message: "object changed to an older watermark".into(),
                            },
                        );
                    }
                    _ => unreachable!("checked object update count and state"),
                };

                if f_delete {
                    ::tokio::time::timeout(
                        ::poprako_obj_dept::actor::REMOTE_TIMEOUT,
                        pool.del(&physical_key),
                    )
                    .await
                    .map_err(|_| {
                        ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                            message: "object delete timed out".into(),
                        }
                    })??;
                }

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "check task is newer than object state".into(),
            }),
            (
                ::poprako_obj_dept::model::task::DELETE,
                ::poprako_obj_dept::actor::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::REMOTE_TIMEOUT,
                    pool.del(&physical_key),
                )
                .await
                .map_err(|_| {
                    ::poprako_obj_dept::rest::ObjDeptError::Retryable {
                        message: "object delete timed out".into(),
                    }
                })??;

                Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete)
            }
            (
                ::poprako_obj_dept::model::task::DELETE,
                ::poprako_obj_dept::actor::ObjKeyState::Pending
                | ::poprako_obj_dept::actor::ObjKeyState::Verified
                | ::poprako_obj_dept::actor::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "delete task targets current object".into(),
            }),
            _ => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "unknown object task operation".into(),
            }),
        }
    }};
}
