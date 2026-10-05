use super::*;

#[test]
fn fourth_failure_becomes_dead() {
    let dispatch_flow = enforce_retry_limit(
        DispatchFlow::Retry {
            err_msg: "failed".into(),
        },
        3,
    );

    assert!(matches!(dispatch_flow, DispatchFlow::Dead { .. }));
}

#[test]
fn first_three_failures_remain_retryable() {
    for retried_count in 0..3 {
        let dispatch_flow = enforce_retry_limit(
            DispatchFlow::Retry {
                err_msg: "failed".into(),
            },
            retried_count,
        );

        assert!(matches!(dispatch_flow, DispatchFlow::Retry { .. }));
    }
}

#[test]
fn waiting_does_not_consume_retry_limit() {
    let dispatch_flow = enforce_retry_limit(
        DispatchFlow::Wait {
            err_msg: "external state is pending".into(),
        },
        i64::MAX,
    );

    assert!(matches!(dispatch_flow, DispatchFlow::Wait { .. }));
}
