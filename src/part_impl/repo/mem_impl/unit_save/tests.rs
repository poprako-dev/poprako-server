#![allow(
    clippy::unwrap_used,
    reason = "Test assertions fail immediately when cache invariants are violated."
)]

// save_cache(UnitSaveCache)(positive): clones serialize identical keys and share results.
// save_cache(UnitSaveCache)(negative): cancellation releases exclusion without a token.
// save_cache(UnitSaveCache)(negative): eviction never removes in-flight exclusion.

use std::time::Duration;

use super::{
    MAX_CACHE_WEIGHT, MAX_TOKENS, MIN_ENTRY_WEIGHT, UnitSaveCache, entry_weight,
};
use crate::part::repo::unit_save::UnitSaveGuard as _;
use crate::value::unit_save::UnitSaveCreatedUnitId;
use std::sync::Arc;
use uuid::Uuid;

fn key(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

#[tokio::test]
async fn cloned_cache_waits_for_publication() {
    let cache = UnitSaveCache::default();

    let cloned = cache.clone();

    let guard = cache.lock_save(key(999)).await;

    assert!(guard.created_unit_ids().is_none());

    assert!(
        tokio::time::timeout(
            Duration::from_millis(20),
            cloned.lock_save(key(999)),
        )
        .await
        .is_err()
    );

    guard.record(Arc::from([]));

    drop(guard);

    let replay = cloned.lock_save(key(999)).await;

    assert!(replay.created_unit_ids().is_some());

    drop(replay);

    assert_eq!(cloned.token_count(), 1);
}

#[tokio::test]
async fn cancelled_owner_releases_key_without_token() {
    let cache = UnitSaveCache::default();

    let cloned = cache.clone();

    let (ready_send, ready_recv) = tokio::sync::oneshot::channel();

    let owner = tokio::spawn(async move {
        let _guard = cloned.lock_save(key(999)).await;

        ready_send.send(()).unwrap();

        std::future::pending::<()>().await;
    });

    ready_recv.await.unwrap();

    owner.abort();

    assert!(owner.await.unwrap_err().is_cancelled());

    let guard =
        tokio::time::timeout(Duration::from_secs(1), cache.lock_save(key(999)))
            .await
            .unwrap();

    assert!(guard.created_unit_ids().is_none());

    drop(guard);
}

#[tokio::test]
async fn capacity_eviction_preserves_lock_and_allows_a_later_miss() {
    let cache = UnitSaveCache::new(16 * MIN_ENTRY_WEIGHT);

    let guard = cache.lock_save(key(999)).await;

    guard.record(Arc::from([]));

    for index in 0..256 {
        cache.tokens.insert(key(index), Arc::from([]));
    }

    assert!(cache.token_count() <= 16);

    // Explicit eviction makes the coordination assertion deterministic.
    cache.tokens.invalidate(&key(999));

    assert!(guard.created_unit_ids().is_none());

    assert!(
        tokio::time::timeout(
            Duration::from_millis(20),
            cache.lock_save(key(999)),
        )
        .await
        .is_err()
    );

    drop(guard);

    assert!(cache.lock_save(key(999)).await.created_unit_ids().is_none());
}

#[tokio::test]
async fn distinct_tokens_and_restarted_cache_are_independent() {
    let cache = UnitSaveCache::default();

    cache.lock_save(key(1)).await.record(Arc::from([]));

    assert!(cache.lock_save(key(2)).await.created_unit_ids().is_none());

    let restarted = UnitSaveCache::default();

    assert!(
        restarted
            .lock_save(key(1))
            .await
            .created_unit_ids()
            .is_none()
    );
}

// Large mappings consume more retention capacity than token-only saves.
#[tokio::test]
async fn weighted_budget_evicts_large_mappings() {
    let cache = UnitSaveCache::new(160 * MIN_ENTRY_WEIGHT);

    for index in 0..256 {
        let pair = UnitSaveCreatedUnitId {
            local_id: "l".repeat(2000),
            unit_id: "unit".into(),
        };

        cache.lock_save(key(index)).await.record(Arc::from([pair]));
    }

    cache.tokens.run_pending_tasks();

    assert!(cache.tokens.weighted_size() <= 160 * MIN_ENTRY_WEIGHT);

    assert!((1..160).contains(&cache.token_count()));
}

// The minimum entry charge bounds count, while string capacity bounds payload weight.
#[test]
fn weights_account_for_allocations_and_entry_count() {
    let empty = entry_weight(&[]);

    assert!(MAX_CACHE_WEIGHT / u64::from(empty) <= MAX_TOKENS);

    let mut local_id = String::with_capacity(4096);

    local_id.push('l');

    let pair = UnitSaveCreatedUnitId {
        local_id,
        unit_id: "unit".into(),
    };

    assert!(entry_weight(&[pair]) >= empty + 4096 - 320);
}
