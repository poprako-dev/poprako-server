//! RDB-backed object-task classification and typed dispatch.

use std::cmp::Ordering;

use crate::rdb_impl::ObjRdbRow;
use crate::rest::{ObjDeptError, ObjDeptRest};

/// Maximum duration of one remote request.
pub const REMOTE_TIMEOUT: std::time::Duration =
    std::time::Duration::from_secs(45);

/// Relative position of a task key against the latest object row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjKeyState {
    //
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

/// Expands one typed RDB object handler.
#[doc(hidden)]
#[macro_export]
// Expands the typed RDB object handler selected by manifest dispatch.
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
        let state = ::poprako_obj_dept::actor::rdb_impl::classify(
            key.version,
            row.as_ref(),
        )?;

        drop(conn);

        match (task.oper.as_str(), state) {
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Verified,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Complete),
            (
                ::poprako_obj_dept::model::task::CHECK,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
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
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Pending,
            ) => {
                let f_exists = ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
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

                        Some(::poprako_obj_dept::actor::rdb_impl::classify(
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
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                            | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                            | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
                        ),
                    ) => true,
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Verified,
                        ),
                    ) => false,
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Pending,
                        ),
                    ) => {
                        return Ok(
                            ::poprako_obj_dept::model::task::ObjTaskAction::Retry {
                                message: "object changed during check".into(),
                            },
                        );
                    }
                    (
                        0,
                        Some(
                            ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
                        ),
                    ) => {
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
                        ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
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
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "check task is newer than object state".into(),
            }),
            (
                ::poprako_obj_dept::model::task::DELETE,
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Missing
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Stale
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Retired,
            ) => {
                ::tokio::time::timeout(
                    ::poprako_obj_dept::actor::rdb_impl::REMOTE_TIMEOUT,
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
                ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Pending
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Verified
                | ::poprako_obj_dept::actor::rdb_impl::ObjKeyState::Future,
            ) => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "delete task targets current object".into(),
            }),
            _ => Ok(::poprako_obj_dept::model::task::ObjTaskAction::Operator {
                message: "unknown object task operation".into(),
            }),
        }
    }};
}
