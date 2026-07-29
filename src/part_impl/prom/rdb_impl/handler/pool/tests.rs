use super::*;

// topic_worker_assignment_is_stable(topic_worker_index)(positive): repeated messages for one topic must stay on one serial worker.
// current_topics_use_distinct_workers(topic_worker_index)(positive): the current topic set should use three of the four workers.

#[test]
fn topic_worker_assignment_is_stable() {
    //
    let first_worker = topic_worker_index("image");

    let second_worker = topic_worker_index("image");

    assert_eq!(first_worker, second_worker);
}

#[test]
fn current_topics_use_distinct_workers() {
    //
    let mut worker_indices = vec![
        topic_worker_index("image"),
        topic_worker_index("check_chapter_upload_finish"),
        topic_worker_index("purge_expired_invitation"),
    ];

    worker_indices.sort_unstable();

    worker_indices.dedup();

    assert_eq!(worker_indices.len(), 3);
}
