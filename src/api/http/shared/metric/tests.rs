use super::*;

use std::time::Duration;

// read(read)(positive): current window buckets are merged by status and matched path.
// read(read)(negative): expired, future, and overwritten buckets are excluded.
// record(record)(positive): an unmatched response changes only the total count.

#[test]
fn read_merges_current_window_buckets() {
    //
    let metric_window = MetricWindow::new();

    metric_window.record(
        100,
        200,
        Some("/api/v1/users/{user_id}"),
        Duration::from_millis(10),
    );

    metric_window.record(
        101,
        422,
        Some("/api/v1/users/{user_id}"),
        Duration::from_millis(20),
    );

    metric_window.record(
        101,
        500,
        Some("/api/v1/comics/{comic_id}"),
        Duration::from_millis(30),
    );

    let metric_total = metric_window.read(101);

    assert_eq!(metric_total.total, 3);

    assert_eq!(metric_total.average_latency_ms, 20.0);

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

    assert_eq!(metric_total.minutes.len(), RECENT_MINUTE_COUNT);

    let current_minute = metric_total
        .minutes
        .iter()
        .find(|metric_minute| metric_minute.minute == 101)
        .expect("current minute should be present");

    assert_eq!(current_minute.total, 2);

    assert_eq!(current_minute.average_latency_ms, 25.0);
}

#[test]
fn read_excludes_buckets_outside_current_window() {
    //
    let metric_window = MetricWindow::new();

    metric_window.record(39, 200, Some("/expired"), Duration::from_millis(10));

    metric_window.record(40, 200, Some("/expired"), Duration::from_millis(10));

    metric_window.record(100, 200, Some("/current"), Duration::from_millis(10));

    metric_window.record(161, 200, Some("/future"), Duration::from_millis(10));

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

    metric_window.record(100, 404, None, Duration::from_millis(10));

    let metric_total = metric_window.read(100);

    assert_eq!(metric_total.total, 1);

    assert_eq!(metric_total.by_error.get(&404), Some(&1));

    assert!(metric_total.by_path.is_empty());
}
