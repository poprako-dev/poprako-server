use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::MatchedPath;
use axum::response::Response;
use serde::Serialize;
#[cfg(feature = "swagger-ui")]
use utoipa::ToSchema;

#[cfg(test)]
mod tests {
    use super::*;

    // read(read)(positive): current window buckets are merged by status and matched path.
    // read(read)(negative): expired, future, and overwritten buckets are excluded.
    // record(record)(positive): an unmatched response changes only the total count.

    #[test]
    fn read_merges_current_window_buckets() {
        //
        let metric_window = MetricWindow::new();

        metric_window.record(100, 200, Some("/api/v1/users/{user_id}"));

        metric_window.record(101, 422, Some("/api/v1/users/{user_id}"));

        metric_window.record(101, 500, Some("/api/v1/comics/{comic_id}"));

        let metric_total = metric_window.read(101);

        assert_eq!(metric_total.total, 3);

        assert_eq!(metric_total.by_error.get(&422), Some(&1));

        assert_eq!(metric_total.by_error.get(&500), Some(&1));

        assert_eq!(
            metric_total.by_path.get("/api/v1/users/{user_id}"),
            Some(&2),
        );

        assert_eq!(
            metric_total.by_path.get("/api/v1/comics/{comic_id}"),
            Some(&1),
        );
    }

    #[test]
    fn read_excludes_buckets_outside_current_window() {
        //
        let metric_window = MetricWindow::new();

        metric_window.record(39, 200, Some("/expired"));

        metric_window.record(40, 200, Some("/expired"));

        metric_window.record(100, 200, Some("/current"));

        metric_window.record(161, 200, Some("/future"));

        let metric_total = metric_window.read(100);

        assert_eq!(metric_total.total, 1);

        assert_eq!(metric_total.by_path.get("/current"), Some(&1));

        assert!(!metric_total.by_path.contains_key("/expired"));

        assert!(!metric_total.by_path.contains_key("/future"));
    }

    #[test]
    fn record_without_matched_path_changes_only_total() {
        //
        let metric_window = MetricWindow::new();

        metric_window.record(100, 404, None);

        let metric_total = metric_window.read(100);

        assert_eq!(metric_total.total, 1);

        assert_eq!(metric_total.by_error.get(&404), Some(&1));

        assert!(metric_total.by_path.is_empty());
    }
}

const BUCKET_COUNT: usize = 60;
const SECONDS_PER_BUCKET: u64 = 60;

static METRIC_WINDOW: LazyLock<MetricWindow> = LazyLock::new(MetricWindow::new);

/// Aggregate metrics for the current time window.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger-ui", derive(ToSchema))]
pub(crate) struct MetricTotal {
    pub(crate) total: u64,
    pub(crate) by_error: HashMap<u16, u64>,
    pub(crate) by_path: HashMap<String, u64>,
}

#[derive(Default)]
struct MetricBucket {
    minute: u64,
    total: u64,
    by_error: HashMap<u16, u64>,
    by_path: HashMap<String, u64>,
}

struct MetricWindow {
    buckets: [Mutex<MetricBucket>; BUCKET_COUNT],
}

impl MetricTotal {
    fn new() -> Self {
        Self {
            total: 0,
            by_error: HashMap::new(),
            by_path: HashMap::new(),
        }
    }
}

impl MetricBucket {
    fn reset(&mut self, minute: u64) {
        //
        self.minute = minute;

        self.total = 0;

        self.by_error.clear();

        self.by_path.clear();
    }
}

impl MetricWindow {
    fn new() -> Self {
        Self {
            buckets: std::array::from_fn(|_| {
                Mutex::new(MetricBucket::default())
            }),
        }
    }

    fn record(&self, minute: u64, status: u16, matched_path: Option<&str>) {
        //
        let bucket_index = minute as usize % BUCKET_COUNT;

        let mut bucket = lock_bucket(&self.buckets[bucket_index]);

        match bucket.minute == minute {
            //
            true => {}

            false => bucket.reset(minute),
        }

        bucket.total = bucket.total.saturating_add(1);

        if status >= 400 {
            //
            let error_total = bucket.by_error.entry(status).or_default();

            *error_total = error_total.saturating_add(1);
        }

        let Some(matched_path) = matched_path else {
            return;
        };

        let path_total =
            bucket.by_path.entry(matched_path.to_owned()).or_default();

        *path_total = path_total.saturating_add(1);
    }

    fn read(&self, minute: u64) -> MetricTotal {
        //
        let mut metric_total = MetricTotal::new();

        for bucket in &self.buckets {
            //
            let bucket = lock_bucket(bucket);

            let bucket_age = minute.saturating_sub(bucket.minute);

            if bucket.minute > minute || bucket_age >= BUCKET_COUNT as u64 {
                continue;
            }

            metric_total.total =
                metric_total.total.saturating_add(bucket.total);

            merge_totals(&mut metric_total.by_error, &bucket.by_error);

            merge_totals(&mut metric_total.by_path, &bucket.by_path);
        }

        metric_total
    }
}

/// Records one response in the current minute bucket.
///
/// The path dimension is populated only from axum's matched route template.
pub(crate) fn record_response(
    response: &Response,
    matched_path: Option<&MatchedPath>,
) {
    //
    let minute = current_minute();

    let status = response.status().as_u16();

    let matched_path = matched_path.map(MatchedPath::as_str);

    METRIC_WINDOW.record(minute, status, matched_path);
}

/// Returns an approximate snapshot covering the current and previous 59 minutes.
pub(crate) fn read_total() -> MetricTotal {
    METRIC_WINDOW.read(current_minute())
}

fn current_minute() -> u64 {
    //
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    elapsed.as_secs() / SECONDS_PER_BUCKET
}

fn lock_bucket(bucket: &Mutex<MetricBucket>) -> MutexGuard<'_, MetricBucket> {
    match bucket.lock() {
        //
        Ok(bucket) => bucket,

        Err(poisoned) => poisoned.into_inner(),
    }
}

fn merge_totals<K>(target: &mut HashMap<K, u64>, source: &HashMap<K, u64>)
where
    K: Clone + Eq + Hash,
{
    for (key, value) in source {
        //
        let total = target.entry(key.clone()).or_default();

        *total = total.saturating_add(*value);
    }
}
