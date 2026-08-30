//! Physical object-storage contract used by ObjDept.

use std::future::Future;

use url::Url;

use crate::model::slot::ObjPoolSlot;
use crate::rest::ObjDeptRest;

/// Read-only physical object operations.
pub trait ObjPoolView {
    /// Generates a public or signed read URL for one physical key.
    fn gen_url(
        &self,
        key: &str,
    ) -> impl Future<Output = ObjDeptRest<Url>> + Send;

    /// Checks whether one physical key exists.
    fn has(&self, key: &str) -> impl Future<Output = ObjDeptRest<bool>> + Send;
}

/// Complete storage-neutral physical object operations.
pub trait ObjPool: ObjPoolView {
    /// Generates a write capability for one physical key.
    fn gen_slot(
        &self,
        key: &str,
        content_type: &str,
        byte_len: u64,
    ) -> impl Future<Output = ObjDeptRest<ObjPoolSlot>> + Send;

    /// Deletes one physical key idempotently.
    fn del(&self, key: &str) -> impl Future<Output = ObjDeptRest<()>> + Send;
}
