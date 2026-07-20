use super::*;

#[test]
fn incomplete_uploads_are_retried() {
    assert!(matches!(resolve_task_flow(Ok(false)), TaskFlow::Retry(_)));
}

#[test]
fn resolved_uploads_are_completed() {
    assert!(matches!(resolve_task_flow(Ok(true)), TaskFlow::Complete));
}
