use std::collections::HashMap;
use std::hash::Hash;
use std::sync::{LazyLock, Mutex, MutexGuard};
use std::time::{SystemTime, UNIX_EPOCH};

use axum::extract::MatchedPath;
use axum::response::Response;
use serde::Serialize;
#[cfg(feature = "swagger")]
use utoipa::ToSchema;

#[cfg(test)]
mod tests;

const BUCKET_COUNT: usize = 60;
const RECENT_MINUTE_COUNT: usize = 30;
const SECONDS_PER_BUCKET: u64 = 60;

static METRIC_WINDOW: LazyLock<MetricWindow> = LazyLock::new(MetricWindow::new);

/// Aggregate metrics for the current time window.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub(crate) struct MetricTotal {
    pub(crate) total: u64,
    pub(crate) average_latency_ms: f64,

    #[serde(skip)]
    total_latency_micros: u64,

    pub(crate) by_error: HashMap<u16, u64>,
    pub(crate) by_path: HashMap<String, u64>,
    pub(crate) minutes: Vec<MetricMinute>,
}

/// Aggregate metrics for one minute in the recent sliding window.
#[derive(Debug, Serialize)]
#[cfg_attr(feature = "swagger", derive(ToSchema))]
pub(crate) struct MetricMinute {
    pub(crate) minute: u64,
    pub(crate) total: u64,
    pub(crate) average_latency_ms: f64,
}

#[derive(Default)]
struct MetricBucket {
    minute: u64,
    total: u64,
    total_latency_micros: u64,
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
            average_latency_ms: 0.0,
            total_latency_micros: 0,
            by_error: HashMap::new(),
            by_path: HashMap::new(),
            minutes: Vec::new(),
        }
    }
}

impl MetricBucket {
    fn reset(&mut self, minute: u64) {
        //
        self.minute = minute;

        self.total = 0;

        self.total_latency_micros = 0;

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

    fn record(
        &self,
        minute: u64,
        status: u16,
        matched_path: Option<&str>,
        latency: std::time::Duration,
    ) {
        //
        let bucket_index = minute as usize % BUCKET_COUNT;

        let mut bucket = lock_bucket(&self.buckets[bucket_index]);

        match bucket.minute == minute {
            //
            true => {}

            false => bucket.reset(minute),
        }

        bucket.total = bucket.total.saturating_add(1);

        let latency_micros = latency.as_micros().try_into().unwrap_or(u64::MAX);

        bucket.total_latency_micros =
            bucket.total_latency_micros.saturating_add(latency_micros);

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
}

impl MetricMinute {
    fn from_bucket(minute: u64, bucket: &MetricBucket) -> Self {
        //
        let (total, total_latency_micros) = match bucket.minute == minute {
            //
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

/// Records one response in the current minute bucket.
///
/// The path dimension is populated only from axum's matched route template.
pub(crate) fn record_response(
    response: &Response,
    matched_path: Option<&MatchedPath>,
    latency: std::time::Duration,
) {
    //
    let minute = curr_minute();

    let status = response.status().as_u16();

    let matched_path = matched_path.map(MatchedPath::as_str);

    METRIC_WINDOW.record(minute, status, matched_path, latency);
}

/// Returns an approximate snapshot covering the current and previous 59 minutes.
pub(crate) fn read_total() -> MetricTotal {
    METRIC_WINDOW.read(curr_minute())
}

fn curr_minute() -> u64 {
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

fn average_latency_ms(total_latency_micros: u64, total: u64) -> f64 {
    match total {
        //
        0 => 0.0,

        _ => total_latency_micros as f64 / total as f64 / 1_000.0,
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
