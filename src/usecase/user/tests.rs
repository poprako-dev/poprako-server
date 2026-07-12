//! Test fixtures and cases for the user use case module.
//!
//! Tests exercise user profile reads, updates, avatar management, activity
//! tracking, and account deletion against a [`Mock`] that doubles as the
//! driver, repository, prom enqueuer, image pool, and effect developer.
//!
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

// get_info(get_info)(positive): a user reading itself should receive info and emit UserActive.
// get_info(get_info)(positive): reading another user should not emit UserActive.
// get_info(get_info)(negative): missing user should propagate an argument error.
// update_info(update_info)(positive): owner update should change user info and member nickname.
// update_info(update_info)(negative): non-owner update should return a permission error without mutation.
// update_info(update_info)(negative): missing user should rollback the transaction.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing user should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching owner and version should mark the avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): non-owner mark should return a permission error.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should rollback uploaded state.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): old reservation replay should fail without marking current avatar uploaded.
// delete(delete)(positive): owner delete should remove user, credentials, and memberships, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a user without an uploaded avatar should not enqueue prom records.
// delete(delete)(negative): non-owner delete should return a permission error without mutation.
// delete(delete)(negative): missing user should rollback state.

use super::*;

use time::OffsetDateTime;

use crate::model::{member_model, user_model};
use crate::part::effect::event::Event;
use crate::part::prom::Payload;
use crate::part::prom::task::{ImageKind, ImageTask};
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::{credential, user};
use crate::test_util::{
    assert_expected_message, assert_expected_variant,
    assert_one_image_check_record,
};
use crate::value::role::{RoleField, RoleMask};

/// Builds a [`UserInfo`] fixture with avatar fields set.
fn user_with_avatar(
    id: &str,
    qid: &str,
    nickname: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: i64,
) -> user_model::Info {
    user_model::Info {
        avatar_key: Some(avatar_key.into()),
        avatar_uploaded,
        avatar_version,
        ..user(id, qid, nickname)
    }
}

/// Builds a [`MemberInfo`] fixture.
fn member(
    id: &str,
    user_id: &str,
    user_nickname: &str,
    team_id: &str,
) -> member_model::Info {
    member_model::Info {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::ADMIN),
    }
}

/// Builds a [`UserToken`] fixture for the given user ID.
fn token(user_id: &str) -> user_model::Token {
    user_model::Token {
        user_id: user_id.into(),
    }
}

/// Builds an [`UpdateUserInfoData`] fixture.
fn update_data(
    id: &str,
    qid: &str,
    nickname: &str,
) -> user_data::UpdateInfoData {
    user_data::UpdateInfoData {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
    }
}

/// Builds a [`ReserveUserAvatarData`] fixture.
fn reserve_data(file_ext: &str) -> user_data::ReserveAvatarData {
    user_data::ReserveAvatarData {
        file_ext: file_ext.into(),
    }
}

/// Builds a [`MarkUserAvatarUploadedData`] fixture.
fn mark_data(avatar_version: i64) -> user_data::MarkAvatarUploadedData {
    user_data::MarkAvatarUploadedData { avatar_version }
}

/// Counts [`Delete`](ImageTask::Delete) prom records matching the given object key.
fn count_delete_records(records: &[MockPromRecord], object_key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.payload(),
                Payload::Image(ImageTask::Delete { object_key: key })
                    if key == object_key
            )
        })
        .count()
}

#[tokio::test]
async fn get_info_emits_active_for_self() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let val = get_info(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .unwrap();

    assert_eq!(val.id, "user-1");

    assert_eq!(val.nickname, "Nick");

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    let Event::UserActive(payload) = &events[0] else {
        panic!("expected UserActive event");
    };

    assert_eq!(payload.user_id, "user-1");
}

#[tokio::test]
async fn get_info_does_not_emit_active_for_other_user() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-2", "qid-2", "Other"),
        credential("user-2", "password"),
    );

    get_info(&mock, &mock, &mock, token("user-1"), "user-2".into())
        .await
        .unwrap();

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn get_info_propagates_missing_user() {
    //
    let mock = Mock::new();

    let err = get_info(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_info_updates_user_and_member_nickname() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "password"),
    );

    mock.seed_member(member("member-1", "user-1", "Old", "team-1"));

    update_info(
        &mock,
        &mock,
        token("user-1"),
        update_data("user-1", "qid-new", "New"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users[0].qid, "qid-new");

    assert_eq!(snapshot.users[0].nickname, "New");

    assert_eq!(snapshot.members[0].user_nickname, "New");
}

#[tokio::test]
async fn update_info_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "password"),
    );

    let err = update_info(
        &mock,
        &mock,
        token("user-2"),
        update_data("user-1", "qid-new", "New"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users[0].qid, "qid-1");

    assert_eq!(snapshot.users[0].nickname, "Old");
}

#[tokio::test]
async fn update_info_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let err = update_info(
        &mock,
        &mock,
        token("user-1"),
        update_data("user-1", "qid-new", "New"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}

#[tokio::test]
async fn reserve_avatar_updates_state_enqueues_check_and_returns_put_url() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let val = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("png"),
    )
    .await
    .unwrap();

    assert_eq!(val.avatar_version, 1);

    assert_eq!(
        val.put_url,
        "https://test.local/put/user_avatar/user-1-1.png"
    );

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.users[0].avatar_key.as_deref(),
        Some("user_avatar/user-1-1.png")
    );

    assert!(!snapshot.users[0].avatar_uploaded);

    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::UserAvatar,
        "user-1",
        "user_avatar/user-1-1.png",
        1,
    );
}

#[tokio::test]
async fn reserve_avatar_replacing_avatar_enqueues_delete_and_check() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "old-key", true, 1),
        credential("user-1", "password"),
    );

    reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("jpg"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(count_delete_records(&snapshot.prom_records, "old-key"), 1);

    assert_one_image_check_record(
        &snapshot.prom_records,
        ImageKind::UserAvatar,
        "user-1",
        "user_avatar/user-1-2.jpg",
        2,
    );
}

#[tokio::test]
async fn reserve_avatar_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let err = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn reserve_avatar_propagates_put_url_failure_after_commit() {
    //
    let mock = Mock::new().with_image_put_failure();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let err = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("png"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.users[0].avatar_key.as_deref(),
        Some("user_avatar/user-1-1.png")
    );

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn mark_avatar_uploaded_marks_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "user-1".into(),
        mark_data(2),
    )
    .await
    .unwrap();

    assert!(mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_accepts_repeated_matching_version() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let first = mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "user-1".into(),
        mark_data(2),
    )
    .await;

    assert!(first.is_ok());

    let second = mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "user-1".into(),
        mark_data(2),
    )
    .await;

    assert!(second.is_ok());

    assert!(mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_non_owner() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let err = mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-2"),
        "user-1".into(),
        mark_data(2),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert!(!mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rolls_back_stale_version() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let err = mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "user-1".into(),
        mark_data(1),
    )
    .await
    .err()
    .unwrap();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-avatar-upload",
    );

    assert!(!mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_old_reservation_replay() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "old-key", true, 1),
        credential("user-1", "password"),
    );

    let reserved = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("png"),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(reserved.avatar_version, 2);

    let err = mark_avatar_uploaded(
        &mock,
        &mock,
        token("user-1"),
        "user-1".into(),
        mark_data(1),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-avatar-upload",
    );

    assert!(!snapshot.users[0].avatar_uploaded);

    assert_eq!(snapshot.users[0].avatar_version, 2);
}

#[tokio::test]
async fn delete_removes_user_credentials_members_and_enqueues_avatar_delete() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", true, 2),
        credential("user-1", "password"),
    );

    mock.seed_member(member("member-1", "user-1", "Nick", "team-1"));

    mock.seed_member(member("member-2", "user-1", "Nick", "team-2"));

    delete(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.users.is_empty());

    assert!(snapshot.credentials.is_empty());

    assert!(snapshot.members.is_empty());

    assert_eq!(
        count_delete_records(&snapshot.prom_records, "avatar-key"),
        1
    );
}

#[tokio::test]
async fn delete_without_uploaded_avatar_does_not_enqueue_prom() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", false, 2),
        credential("user-1", "password"),
    );

    delete(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_rejects_non_owner_without_mutation() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let err = delete(&mock, &mock, &mock, token("user-2"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.users.len(), 1);

    assert_eq!(snapshot.credentials.len(), 1);
}

#[tokio::test]
async fn delete_rolls_back_missing_user() {
    //
    let mock = Mock::new();

    let err = delete(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}
