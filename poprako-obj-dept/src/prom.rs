//! Durable Check/Delete contracts used by ObjDept.

use std::future::Future;

use time::OffsetDateTime;

use crate::key::ObjKey;
use crate::model::task::ObjPromTask;
use crate::rest::ObjDeptRest;

/// Actor-side durable task operations.
pub trait ObjProm {
    /// Reclaims expired work across every object topic.
    fn reset_tasks(&self) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Claims the globally oldest visible task.
    fn claim_task(
        &self,
    ) -> impl Future<Output = ObjDeptRest<Option<ObjPromTask>>> + Send;

    /// Completes one exact fenced task.
    fn complete_task(
        &self,
        task: &ObjPromTask,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Returns one exact fenced task to pending.
    fn retry_task<'a>(
        &'a self,
        task: &'a ObjPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Marks one exact fenced task for operator repair.
    fn mark_task_operator<'a>(
        &'a self,
        task: &'a ObjPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;
}

/// Transaction-side durable task creation.
pub trait ObjPromDefer<C> {
    /// Defers one Check task in the caller transaction.
    fn defer_check<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        key: &'a ObjKey,
        expires_at: OffsetDateTime,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers one Delete task in the caller transaction.
    fn defer_delete<'a>(
        &'a self,
        context: &'a mut C,
        topic: &'a str,
        key: &'a ObjKey,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;
}
