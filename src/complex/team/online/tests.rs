use super::*;

#[test]
fn mark_lists_online_users_in_sorted_order() {
    let now = Instant::now();

    mark_user_online_at("online-sort-team", "user-2", now);

    mark_user_online_at("online-sort-team", "user-1", now);

    let online_user_ids = list_online_user_ids_at("online-sort-team", now);

    assert_eq!(online_user_ids, ["user-1", "user-2"]);
}

#[test]
fn repeated_mark_renews_online_lease() {
    let now = Instant::now();

    mark_user_online_at("online-renew-team", "user-1", now);

    let renewed_at = now + Duration::from_secs(9 * 60);

    mark_user_online_at("online-renew-team", "user-1", renewed_at);

    let old_expiration = now + ONLINE_USER_TTL;

    assert_eq!(
        list_online_user_ids_at("online-renew-team", old_expiration),
        ["user-1"]
    );

    let renewed_expiration = renewed_at + ONLINE_USER_TTL;

    assert!(
        list_online_user_ids_at("online-renew-team", renewed_expiration)
            .is_empty()
    );
}

#[test]
fn expiration_removes_empty_team_entry() {
    let now = Instant::now();

    mark_user_online_at("online-expire-team", "user-1", now);

    let expired_at = now + ONLINE_USER_TTL;

    assert!(
        list_online_user_ids_at("online-expire-team", expired_at).is_empty()
    );

    assert!(!ONLINE_USER_DEADLINES.contains_key("online-expire-team"));
}

#[test]
fn teams_keep_independent_online_users() {
    let now = Instant::now();

    mark_user_online_at("online-isolation-team-a", "user-1", now);

    mark_user_online_at("online-isolation-team-b", "user-2", now);

    assert_eq!(
        list_online_user_ids_at("online-isolation-team-a", now),
        ["user-1"]
    );

    assert_eq!(
        list_online_user_ids_at("online-isolation-team-b", now),
        ["user-2"]
    );
}
