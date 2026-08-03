use super::*;

#[test]
fn first_mark_adds_user_and_list_sorts_ids() {
    let online_user_deadlines = DashMap::new();
    let now = Instant::now();

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-sort-team",
        "user-2",
        now,
    );

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-sort-team",
        "user-1",
        now,
    );

    assert_eq!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-sort-team",
            now,
        ),
        ["user-1", "user-2"],
    );
}

#[test]
fn repeated_mark_renews_existing_lease() {
    let online_user_deadlines = DashMap::new();
    let now = Instant::now();

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-renew-team",
        "user-1",
        now,
    );

    let renewed_at = now + Duration::from_secs(60);

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-renew-team",
        "user-1",
        renewed_at,
    );

    assert_eq!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-renew-team",
            now + ONLINE_USER_TTL,
        ),
        ["user-1"],
    );

    assert!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-renew-team",
            renewed_at + ONLINE_USER_TTL,
        )
        .is_empty(),
    );
}

#[test]
fn list_removes_expired_user_and_empty_team() {
    let online_user_deadlines = DashMap::new();
    let now = Instant::now();

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-expire-team",
        "user-1",
        now,
    );

    assert!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-expire-team",
            now + ONLINE_USER_TTL,
        )
        .is_empty(),
    );

    assert!(!online_user_deadlines.contains_key("online-mem-expire-team"));
}

#[test]
fn online_users_remain_isolated_by_team() {
    let online_user_deadlines = DashMap::new();
    let now = Instant::now();

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-team-a",
        "user-a",
        now,
    );

    mark_user_online_at(
        &online_user_deadlines,
        "online-mem-team-b",
        "user-b",
        now,
    );

    assert_eq!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-team-a",
            now,
        ),
        ["user-a"],
    );

    assert_eq!(
        list_online_user_ids_at(
            &online_user_deadlines,
            "online-mem-team-b",
            now,
        ),
        ["user-b"],
    );
}
