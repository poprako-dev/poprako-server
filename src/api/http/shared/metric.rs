use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use axum::extract::MatchedPath;
use axum::response::Response;
use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

#[cfg(test)]
// Shared metric tests stay behind module-private helpers.
mod tests;

// One hour of one-minute buckets gives a full recent-hour window.
const BUCKET_COUNT: usize = 60;

// Recent-graph depth keeps the latest 30 minutes visible to clients.
const RECENT_MINUTE_COUNT: usize = 30;

// Each bucket covers one minute of wall-clock time.
const SECONDS_PER_BUCKET: u64 = 60;

// Sliding metric window singleton shared by HTTP metrics entry points.
static METRIC_WINDOW: LazyLock<MetricWindow> = LazyLock::new(MetricWindow::new);

/// Aggregate metrics for the current time window.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MetricTotal {
    /// Total request count in the current sliding window.
    pub total: u64,
    /// Mean latency across all requests in the window, in milliseconds.
    pub average_latency_ms: f64,

    /// Accumulated latency in microseconds used to compute the average.
    #[serde(skip)]
    total_latency_micros: u64,

    /// Count of 4xx/5xx responses grouped by their HTTP status code.
    pub by_error: HashMap<u16, u64>,
    /// Count of requests grouped by the matched route template.
    pub by_path: HashMap<String, u64>,
    /// Per-minute breakdown for the most recent 30 minutes.
    pub minutes: Vec<MetricMinute>,
}

impl MetricTotal {
    // Builds an empty aggregation snapshot for accumulation.
    fn new() -> Self {
        //
        Self {
            total: 0,
            average_latency_ms: 0.0,
            total_latency_micros: 0,
            by_error: HashMap::new(),
            by_path: HashMap::new(),
            minutes: Vec::new(),
        }
    }
}

/// Aggregate metrics for one minute in the recent sliding window.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub struct MetricMinute {
    /// Unix timestamp truncated to minute granularity.
    pub minute: u64,
    /// Request count recorded in this minute.
    pub total: u64,
    /// Mean latency for requests in this minute, in milliseconds.
    pub average_latency_ms: f64,
}

impl MetricMinute {
    // Builds one minute bucket snapshot from a raw bucket state.
    fn from_bucket(minute: u64, bucket: &MetricBucket) -> Self {
        //
        let (total, total_latency_micros) = match bucket.minute == minute {
            //
            // Keep latency and count aligned with the requested minute.
            true => (bucket.total, bucket.total_latency_micros),

            false => (0, 0),
        };

        Self {
            minute,
            total,
            average_latency_ms: average_latency_ms(total_latency_micros, total),
        }
    }
}

#[derive(Default)]
// One sliding bucket that stores request counters and latency for one minute.
struct MetricBucket {
    // Bucket minute key, used for modulo rotation.
    minute: u64,
    // Total request count in this bucket.
    total: u64,
    // Accumulated latency, in microseconds, for this bucket.
    total_latency_micros: u64,
    // Error status histogram for this bucket.
    by_error: HashMap<u16, u64>,
    // Path histogram for this bucket.
    by_path: HashMap<String, u64>,
}

impl MetricBucket {
    // Resets a bucket for a new minute before new writes.
    fn reset(&mut self, minute: u64) {
        //
        self.minute = minute;

        self.total = 0;

        self.total_latency_micros = 0;

        self.by_error.clear();

        self.by_path.clear();
    }
}

// Sliding window composed of fixed buckets rotated by minute index.
struct MetricWindow {
    // One slot per bucket index, protected by a mutex for concurrency safety.
    buckets: [Mutex<MetricBucket>; BUCKET_COUNT],
}

impl MetricWindow {
    // Creates an initialized window with zeroed buckets.
    fn new() -> Self {
        //
        Self {
            buckets: std::array::from_fn(|_| {
                Mutex::new(MetricBucket::default())
            }),
        }
    }

    // Builds a full window snapshot and fills all minute buckets.
    fn read_recent_minutes(&self, minute: u64) -> Vec<MetricMinute> {
        //
        let first_minute =
            minute.saturating_sub(RECENT_MINUTE_COUNT as u64 - 1);

        (first_minute..=minute)
            .map(|window_minute| {
                //
                let bucket_index = window_minute as usize % BUCKET_COUNT;

                let bucket = lock_bucket(&self.buckets[bucket_index]);

                MetricMinute::from_bucket(window_minute, &bucket)
            })
            .collect()
    }

    // Records one request into the correct minute slot.
    fn record(
        &self,
        minute: u64,
        status: u16,
        matched_path: Option<&str>,
        latency: Duration,
    ) {
        //
        let bucket_index = minute as usize % BUCKET_COUNT;

        let mut bucket = lock_bucket(&self.buckets[bucket_index]);

        if bucket.minute != minute {
            bucket.reset(minute);
        }

        bucket.total = bucket.total.saturating_add(1);

        let latency_micros = latency.as_micros().try_into().unwrap_or(u64::MAX);

        bucket.total_latency_micros =
            bucket.total_latency_micros.saturating_add(latency_micros);

        if status >= 400 {
            //
            // Count bad/failed statuses for quick error visibility.
            let err_total = bucket.by_error.entry(status).or_default();

            *err_total = err_total.saturating_add(1);
        }

        let Some(matched_path) = matched_path else {
            return;
        };

        let path_total =
            bucket.by_path.entry(matched_path.to_owned()).or_default();

        *path_total = path_total.saturating_add(1);
    }

    // Reads and aggregates the current 60-minute window values.
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

            metric_total.total_latency_micros = metric_total
                .total_latency_micros
                .saturating_add(bucket.total_latency_micros);

            merge_totals(&mut metric_total.by_error, &bucket.by_error);

            merge_totals(&mut metric_total.by_path, &bucket.by_path);
        }

        metric_total.average_latency_ms = average_latency_ms(
            metric_total.total_latency_micros,
            metric_total.total,
        );

        metric_total.minutes = self.read_recent_minutes(minute);

        metric_total
    }
}

/// Records one response in the current minute bucket.
///
/// The path dimension is populated only from axum's matched route template.
pub fn record_response(
    response: &Response,
    matched_path: Option<&MatchedPath>,
    latency: Duration,
) {
    //
    let minute = curr_minute();

    let status = response.status().as_u16();

    let matched_path = matched_path.map(MatchedPath::as_str);

    METRIC_WINDOW.record(minute, status, matched_path, latency);
}

/// Returns an approximate snapshot covering the current and previous 59 minutes.
pub fn read_total() -> MetricTotal {
    METRIC_WINDOW.read(curr_minute())
}

// Gets the current minute bucket key from UNIX epoch seconds.
fn curr_minute() -> u64 {
    //
    let elapsed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();

    elapsed.as_secs() / SECONDS_PER_BUCKET
}

// Locks and returns the metric bucket mutex, handling poisoned states.
fn lock_bucket(bucket: &Mutex<MetricBucket>) -> MutexGuard<'_, MetricBucket> {
    //
    match bucket.lock() {
        //
        Ok(bucket) => bucket,

        Err(poisoned) => poisoned.into_inner(),
    }
}

// Converts total latency microseconds into a millisecond average.
fn average_latency_ms(total_latency_micros: u64, total: u64) -> f64 {
    //
    match total {
        //
        0 => 0.0,

        _ => total_latency_micros as f64 / total as f64 / 1_000.0,
    }
}

// Merges per-key counters from one map into another without losing existing values.
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
