use super::pool::enforce_retry_limit;
use super::task_flow::TaskFlow;

#[test]
fn fourth_failure_becomes_dead() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Retry {
            err_message: "failed".into(),
        },
        3,
    );

    assert!(matches!(task_flow, TaskFlow::Dead { .. }));
}

#[test]
fn first_three_failures_remain_retryable() {
    for retried_count in 0..3 {
        let task_flow = enforce_retry_limit(
            TaskFlow::Retry {
                err_message: "failed".into(),
            },
            retried_count,
        );

        assert!(matches!(task_flow, TaskFlow::Retry { .. }));
    }
}

#[test]
fn waiting_does_not_consume_retry_limit() {
    let task_flow = enforce_retry_limit(
        TaskFlow::Wait {
            err_message: "external state is pending".into(),
        },
        i64::MAX,
    );

    assert!(matches!(task_flow, TaskFlow::Wait { .. }));
}
