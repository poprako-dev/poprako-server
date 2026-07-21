use super::*;

#[test]
fn incomplete_uploads_complete_the_one_shot_task() {
    assert!(matches!(resolve_task_flow(Ok(false)), TaskFlow::Complete));
}

#[test]
fn resolved_uploads_are_completed() {
    assert!(matches!(resolve_task_flow(Ok(true)), TaskFlow::Complete));
}
