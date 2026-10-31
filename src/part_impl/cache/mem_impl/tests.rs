#![allow(
    clippy::unwrap_used,
    reason = "Cache contract assertions fail immediately when invariants are violated"
)]

use std::sync::Arc;
use std::time::Duration;

use tokio::time::timeout;
use uuid::Uuid;

use super::MemCache;
use crate::part::cache::issue_save::{
    IssueSaveCache as _, IssueSaveGuard as _,
};
use crate::part::cache::unit_save::{UnitSaveCache as _, UnitSaveGuard as _};

// cache_clone(MemUnitSaveCache)(positive): independently injected handles share exclusion and committed mappings.
#[tokio::test]
async fn cloned_handles_share_unit_exclusion_and_results() {
    let cache = MemCache::default();

    let cloned = cache.clone();

    let save_id = Uuid::new_v4();

    let guard = cache.lock_save(save_id).await;

    assert!(
        timeout(Duration::from_millis(20), cloned.lock_save(save_id))
            .await
            .is_err()
    );

    guard.record(Arc::from([]));

    drop(guard);

    let replay = timeout(Duration::from_secs(1), cloned.lock_save(save_id))
        .await
        .unwrap();

    assert_eq!(replay.created_unit_ids(), Some(Arc::from([])));

    drop(replay);

    let restarted = MemCache::default();

    assert!(
        restarted
            .lock_save(save_id)
            .await
            .created_unit_ids()
            .is_none()
    );
}

// cache_clone(MemIssueSaveCache)(positive): clones share exclusion and receipts independently of Unit saves.
#[tokio::test]
async fn cloned_handles_share_issue_exclusion_and_receipts() {
    let cache = MemCache::default();

    let cloned = cache.clone();

    let save_id = Uuid::new_v4();

    let guard = cache.lock_issue_save(save_id).await;

    assert!(
        timeout(Duration::from_millis(20), cloned.lock_issue_save(save_id))
            .await
            .is_err()
    );

    let unit_guard = timeout(Duration::from_secs(1), cloned.lock_save(save_id))
        .await
        .unwrap();

    assert!(unit_guard.created_unit_ids().is_none());

    drop(unit_guard);

    guard.record(crate::value::issue_save::IssueSaveReceipt {
        fingerprint: [7; 32],
        created_issue_ids: Arc::from([]),
    });

    drop(guard);

    let replay =
        timeout(Duration::from_secs(1), cloned.lock_issue_save(save_id))
            .await
            .unwrap();

    assert_eq!(
        replay.receipt().map(|receipt| receipt.fingerprint),
        Some([7; 32])
    );

    drop(replay);

    let restarted = MemCache::default();

    assert!(restarted.lock_issue_save(save_id).await.receipt().is_none());
}

// cache_cancel(MemIssueSaveCache)(negative): cancelling a holder releases exclusion without recording success.
#[tokio::test]
async fn cancelled_issue_holder_releases_exclusion_without_receipt() {
    let cache = MemCache::default();

    let cloned = cache.clone();

    let save_id = Uuid::new_v4();

    let (locked_sink, locked_stream) = tokio::sync::oneshot::channel();

    let task = tokio::spawn(async move {
        let _guard = cloned.lock_issue_save(save_id).await;

        locked_sink.send(()).unwrap();

        std::future::pending::<()>().await;
    });

    timeout(Duration::from_secs(1), locked_stream)
        .await
        .unwrap()
        .unwrap();

    task.abort();

    assert!(task.await.unwrap_err().is_cancelled());

    let retry = timeout(Duration::from_secs(1), cache.lock_issue_save(save_id))
        .await
        .unwrap();

    assert!(retry.receipt().is_none());

    drop(retry);
}
