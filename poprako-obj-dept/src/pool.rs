//! Physical object-storage contract used by ObjDept.

use std::future::Future;

use url::Url;

use crate::model::slot::ObjPoolSlot;
use crate::rest::ObjDeptRest;

/// Storage-neutral physical object operations.
pub trait ObjPool: Clone + Send + Sync + 'static {
    /// Generates a public or signed read URL for one physical key.
    fn gen_url(
        &self,
        key: &str,
    ) -> impl Future<Output = ObjDeptRest<Url>> + Send;

    /// Generates a write capability for one physical key.
    fn gen_slot(
        &self,
        key: &str,
        content_type: &str,
        byte_len: u64,
    ) -> impl Future<Output = ObjDeptRest<ObjPoolSlot>> + Send;

    /// Checks whether one physical key exists.
    fn has(&self, key: &str) -> impl Future<Output = ObjDeptRest<bool>> + Send;

    /// Deletes one physical key idempotently.
    fn del(&self, key: &str) -> impl Future<Output = ObjDeptRest<()>> + Send;
}
