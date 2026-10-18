#![allow(
    clippy::unwrap_used,
    reason = "Test assertions fail immediately when cache invariants are violated."
)]

// save_cache(UnitSaveCache)(positive): clones serialize identical keys and share results.
// save_cache(UnitSaveCache)(negative): cancellation releases exclusion without a token.
// save_cache(UnitSaveCache)(negative): eviction never removes in-flight exclusion.

use std::num::{NonZeroU32, NonZeroU64};
use std::time::Duration;

use super::{UnitSaveCache, entry_weight};
use crate::config::cache::CacheConfig;
use crate::part::repo::unit_save::UnitSaveGuard as _;
use crate::value::unit_save::UnitSaveCreatedUnitId;
use std::sync::Arc;
use uuid::Uuid;

fn key(value: u128) -> Uuid {
    Uuid::from_u128(value)
}

// save_cache(UnitSaveCache)(negative): expiration hides mappings without releasing in-flight exclusion.
#[tokio::test]
async fn expired_token_is_unavailable_while_its_guard_remains_held() {
    let config = CacheConfig {
        unit_save_ttl: Duration::from_millis(100),
        ..CacheConfig::default()
    };

    let cache = UnitSaveCache::new(config);

    let guard = cache.lock_save(key(999)).await;

    guard.record(Arc::from([]));

    assert!(guard.created_unit_ids().is_some());

    tokio::time::sleep(Duration::from_millis(150)).await;

    // Lookup must reject expiration before explicit maintenance is requested.
    assert!(guard.created_unit_ids().is_none());

    assert!(
        tokio::time::timeout(
            Duration::from_millis(20),
            cache.lock_save(key(999)),
        )
        .await
        .is_err()
    );

    assert_eq!(cache.token_count(), 0);

    drop(guard);

    let next = cache.lock_save(key(999)).await;

    next.record(Arc::from([]));

    assert!(next.created_unit_ids().is_some());

    drop(next);
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
    let config = CacheConfig {
        unit_save_limit_mib: NonZeroU32::MIN,
        unit_save_token_budget: NonZeroU64::new(16).unwrap(),
        ..CacheConfig::default()
    };

    let cache = UnitSaveCache::new(config);

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
    let config = CacheConfig {
        unit_save_limit_mib: NonZeroU32::MIN,
        unit_save_token_budget: NonZeroU64::new(160).unwrap(),
        ..CacheConfig::default()
    };

    let cache = UnitSaveCache::new(config);

    for index in 0..256 {
        let pair = UnitSaveCreatedUnitId {
            local_id: "l".repeat(20_000),
            unit_id: "unit".into(),
        };

        cache.lock_save(key(index)).await.record(Arc::from([pair]));
    }

    cache.tokens.run_pending_tasks();

    assert!(cache.tokens.weighted_size() <= config.unit_save_limit_bytes());

    assert!((1..160).contains(&cache.token_count()));
}

// The minimum entry charge bounds count, while string capacity bounds payload weight.
#[test]
fn weights_account_for_allocations_and_entry_count() {
    let config = CacheConfig::default();

    let minimum = config.unit_save_min_entry_weight();

    let empty = entry_weight(&[], minimum);

    assert!(
        config.unit_save_limit_bytes() / u64::from(empty)
            <= config.unit_save_token_budget.get()
    );

    let mut local_id = String::with_capacity(4096);

    local_id.push('l');

    let pair = UnitSaveCreatedUnitId {
        local_id,
        unit_id: "unit".into(),
    };

    assert!(entry_weight(&[pair], minimum) >= empty + 4096 - 320);
}
