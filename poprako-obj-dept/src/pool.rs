//! Physical object-storage contract used by ObjDept.

use std::collections::HashMap;
use std::future::Future;

use futures_util::future::try_join_all;

use crate::model::meta::ObjMeta;
use crate::model::slot::ObjPoolSlot;
use crate::model::url::ObjUrls;
use crate::rest::ObjDeptRest;

/// Static read-URL renditions requested by one object kind.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjUrlProfile {
    //
    /// Exposes only the original object URL.
    OriginOnly,

    /// Exposes the original URL and an image-thumbnail URL when supported.
    ImageThumbnail,
}

/// Read-only physical object operations.
pub trait ObjPoolView {
    /// Generates public or signed read URLs for one physical key and profile.
    fn gen_urls(
        &self,
        key: &str,
        profile: ObjUrlProfile,
    ) -> impl Future<Output = ObjDeptRest<ObjUrls>> + Send;

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

/// Resolves uploaded metadata through a bounded number of pool futures.
#[doc(hidden)]
pub async fn gen_urls_bounded<P, S>(
    pool: &P,
    profile: ObjUrlProfile,
    metas: &HashMap<String, ObjMeta, S>,
) -> ObjDeptRest<HashMap<String, ObjUrls>>
where
    P: ObjPoolView + Sync,
    S: std::hash::BuildHasher + Sync,
{
    // Maximum pool requests resolved concurrently in one batch.
    const CONCURRENCY: usize = 20;

    let mut uploaded = metas
        .iter()
        .filter(|(_, meta)| meta.is_avail)
        .collect::<Vec<_>>();

    uploaded.sort_unstable_by_key(|(id, _)| id.as_str());

    let mut urls = Vec::with_capacity(uploaded.len());

    for chunk in uploaded.chunks(CONCURRENCY) {
        //
        let futures = chunk.iter().map(|(id, meta)| async move {
            //
            let obj_urls = pool.gen_urls(&meta.key.image, profile).await?;

            Ok(((*id).clone(), obj_urls))
        });

        urls.extend(try_join_all(futures).await?);
    }

    Ok(urls.into_iter().collect())
}
