//! Durable Check/Delete contracts used by ObjDept.

use std::future::Future;

use time::OffsetDateTime;

use crate::key::{ObjKey, ObjRegistration};
use crate::model::task::ObjDeptPromTask;
use crate::rest::ObjDeptRest;

/// Actor-side durable task operations.
pub trait ObjDeptProm {
    /// Reclaims expired work across every object topic.
    fn reset_tasks(&self) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Claims the globally oldest visible task.
    fn claim_task(
        &self,
    ) -> impl Future<Output = ObjDeptRest<Option<ObjDeptPromTask>>> + Send;

    /// Deletes a successful task owned by the exact execution credential.
    fn complete_task(
        &self,
        task: &ObjDeptPromTask,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Returns a task owned by the exact execution credential to pending.
    fn retry_task<'a>(
        &'a self,
        task: &'a ObjDeptPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;

    /// Marks a task owned by the exact execution credential for operator repair.
    fn mark_task_operator<'a>(
        &'a self,
        task: &'a ObjDeptPromTask,
        message: &'a str,
    ) -> impl Future<Output = ObjDeptRest<usize>> + Send;
}

/// Transaction-side durable task creation.
///
/// The manifest supplies the topic; producers cannot provide a different one.
///
/// ```compile_fail,E0061
/// use poprako_obj_dept::key::{ObjKey, ObjRegistration};
/// use poprako_obj_dept::prom::ObjDeptPromDefer;
/// async fn defer<C, K: ObjRegistration, P: ObjDeptPromDefer<C>>(
///     prom: &P, context: &mut C, key: &ObjKey<K>,
/// ) {
///     let _ = prom.defer_delete(context, "unrelated_topic", key).await;
/// }
/// ```
///
/// ```
/// use poprako_obj_dept::key::{ObjKey, ObjRegistration};
/// use poprako_obj_dept::prom::ObjDeptPromDefer;
/// async fn defer<C, K: ObjRegistration, P: ObjDeptPromDefer<C>>(
///     prom: &P, context: &mut C, key: &ObjKey<K>,
/// ) {
///     let _ = prom.defer_delete(context, key).await;
/// }
/// ```
pub trait ObjDeptPromDefer<C> {
    /// Defers one Check task in the caller transaction.
    fn defer_check<'a, K: ObjRegistration>(
        &'a self,
        context: &'a mut C,
        key: &'a ObjKey<K>,
        expires_at: OffsetDateTime,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers one Delete task in the caller transaction.
    fn defer_delete<'a, K: ObjRegistration>(
        &'a self,
        context: &'a mut C,
        key: &'a ObjKey<K>,
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers Check tasks in one caller-transaction batch.
    fn defer_checks<'a, K: ObjRegistration>(
        &'a self,
        context: &'a mut C,
        checks: &'a [ObjDeptPromCheck<K>],
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;

    /// Defers Delete tasks in one caller-transaction batch.
    fn defer_deletes<'a, K: ObjRegistration>(
        &'a self,
        context: &'a mut C,
        keys: &'a [ObjKey<K>],
    ) -> impl Future<Output = ObjDeptRest<()>> + Send;
}

/// One Check task requested by a batch object lifecycle operation.
#[derive(PartialEq, Eq)]
pub struct ObjDeptPromCheck<K>
where
    K: ObjRegistration,
{
    /// Logical object generation to verify.
    key: ObjKey<K>,
    /// Time after which absence is treated as a failed upload.
    expires_at: OffsetDateTime,
}

impl<K> Clone for ObjDeptPromCheck<K>
where
    K: ObjRegistration + Clone,
    K::Id: Clone,
{
    // Clones the value when its owned identity supports cloning.
    fn clone(&self) -> Self {
        Self {
            key: self.key.clone(),
            expires_at: self.expires_at,
        }
    }
}

impl<K> ObjDeptPromCheck<K>
where
    K: ObjRegistration,
{
    /// Creates one deferred check for an exact logical object generation.
    #[must_use]
    pub const fn new(key: ObjKey<K>, expires_at: OffsetDateTime) -> Self {
        Self { key, expires_at }
    }

    /// Returns the exact logical object generation to verify.
    #[must_use]
    pub const fn key(&self) -> &ObjKey<K> {
        &self.key
    }

    /// Returns the time after which absence is treated as a failed upload.
    #[must_use]
    pub const fn expires_at(&self) -> OffsetDateTime {
        self.expires_at
    }
}
