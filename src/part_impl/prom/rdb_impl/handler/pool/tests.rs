use super::*;

// topic_worker_assignment_is_stable(topic_worker_index)(positive): repeated messages for one topic must stay on one serial worker.
// current_topics_use_multiple_workers(topic_worker_index)(positive): the current topic set should not collapse onto one worker.

#[test]
fn topic_worker_assignment_is_stable() {
    //
    let first_worker = topic_worker_index("image");

    let second_worker = topic_worker_index("image");

    assert_eq!(first_worker, second_worker);
}

#[test]
fn current_topics_use_multiple_workers() {
    //
    let mut worker_indices = vec![
        topic_worker_index("image"),
        topic_worker_index("advance_raw_provide"),
        topic_worker_index("purge_expired_invitation"),
    ];

    worker_indices.sort_unstable();

    worker_indices.dedup();

    assert!(worker_indices.len() >= 2);
}
