use super::*;

use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;

use crate::result::accept;

#[tokio::test]
async fn collect_bounded_preserves_order_and_limits_concurrency() {
    //
    let active = Arc::new(AtomicUsize::new(0));

    let max_active = Arc::new(AtomicUsize::new(0));

    let futures = (0..25).map(|index| {
        let active = Arc::clone(&active);

        let max_active = Arc::clone(&max_active);

        async move {
            let current = active.fetch_add(1, Ordering::SeqCst) + 1;

            max_active.fetch_max(current, Ordering::SeqCst);

            tokio::time::sleep(Duration::from_millis(5)).await;

            active.fetch_sub(1, Ordering::SeqCst);

            accept(index)
        }
    });

    let values = collect_bounded(futures).await.unwrap();

    assert_eq!(values, (0..25).collect::<Vec<_>>());

    assert_eq!(max_active.load(Ordering::SeqCst), FUTURE_CONCURRENCY_LIMIT);
}
