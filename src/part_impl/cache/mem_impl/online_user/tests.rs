#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

use super::*;

#[test]
fn first_mark_adds_user_and_list_sorts_ids() {
    let online_users = OnlineUserCache::new(Duration::from_secs(90));

    let now = Instant::now();

    online_users.mark_user_online_at("online-mem-sort-team", "user-2", now);

    online_users.mark_user_online_at("online-mem-sort-team", "user-1", now);

    assert_eq!(
        online_users.list_online_user_ids_at("online-mem-sort-team", now),
        ["user-1", "user-2"],
    );
}

#[test]
fn repeated_mark_renews_existing_lease() {
    let online_users = OnlineUserCache::new(Duration::from_secs(90));

    let now = Instant::now();

    online_users.mark_user_online_at("online-mem-renew-team", "user-1", now);

    let renewed_at = now + Duration::from_mins(1);

    online_users.mark_user_online_at(
        "online-mem-renew-team",
        "user-1",
        renewed_at,
    );

    assert_eq!(
        online_users.list_online_user_ids_at(
            "online-mem-renew-team",
            now + Duration::from_secs(90),
        ),
        ["user-1"],
    );

    assert!(
        online_users
            .list_online_user_ids_at(
                "online-mem-renew-team",
                renewed_at + Duration::from_secs(90),
            )
            .is_empty(),
    );
}

#[test]
fn list_removes_expired_user_and_empty_team() {
    let online_users = OnlineUserCache::new(Duration::from_secs(90));

    let now = Instant::now();

    online_users.mark_user_online_at("online-mem-expire-team", "user-1", now);

    assert!(
        online_users
            .list_online_user_ids_at(
                "online-mem-expire-team",
                now + Duration::from_secs(90),
            )
            .is_empty(),
    );

    assert!(
        !online_users
            .deadlines
            .contains_key("online-mem-expire-team")
    );
}

#[test]
fn online_users_remain_isolated_by_team() {
    let online_users = OnlineUserCache::new(Duration::from_secs(90));

    let now = Instant::now();

    online_users.mark_user_online_at("online-mem-team-a", "user-a", now);

    online_users.mark_user_online_at("online-mem-team-b", "user-b", now);

    assert_eq!(
        online_users.list_online_user_ids_at("online-mem-team-a", now),
        ["user-a"],
    );

    assert_eq!(
        online_users.list_online_user_ids_at("online-mem-team-b", now),
        ["user-b"],
    );
}

// cloned_cache_shares_online_users(MemCache::clone)(positive): independently injected handles share process-local leases.
#[tokio::test]
async fn cloned_cache_shares_online_users() {
    let cache = MemCache::default();

    let worker_cache = cache.clone();

    let mark = MarkUserOnline::new("team", "user");

    cache.run(&mark).await.unwrap();

    let list = ListOnlineUserIds::new("team");

    assert_eq!(worker_cache.run(&list).await.unwrap(), ["user"]);
}
