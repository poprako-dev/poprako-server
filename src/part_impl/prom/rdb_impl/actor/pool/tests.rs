use super::*;

// topic_worker_assignment_is_stable(topic_worker_index)(positive): repeated messages for one topic must stay on one serial worker.
// current_topics_use_multiple_workers(topic_worker_index)(positive): the current topic set should not collapse onto one worker.

#[test]
fn topic_worker_assignment_is_stable() {
    //
    let Ok(first_worker) = topic_worker_index("image") else {
        panic!("worker index calculation must succeed");
    };

    let Ok(second_worker) = topic_worker_index("image") else {
        panic!("worker index calculation must succeed");
    };

    assert_eq!(first_worker, second_worker);
}

#[test]
fn current_topics_use_multiple_workers() {
    //
    let worker_results = [
        topic_worker_index("image"),
        topic_worker_index("advance_raw_provide"),
        topic_worker_index("purge_expired_invitation"),
    ];

    let mut worker_indices = worker_results
        .into_iter()
        .map(|result| match result {
            Ok(worker_index) => worker_index,

            Err(_) => panic!("worker index calculation must succeed"),
        })
        .collect::<Vec<_>>();

    worker_indices.sort_unstable();

    worker_indices.dedup();

    assert!(worker_indices.len() >= 2);
}
