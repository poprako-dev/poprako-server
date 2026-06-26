//! Test fixtures and cases for the user use case module.
//!
//! Tests exercise user profile reads, updates, avatar management, activity
//! tracking, and account deletion against a [`Mock`] that doubles as the
//! driver, repository, prom enqueuer, image pool, and effect developer.
//!
//! [`Mock`]: crate::part_impl::repo_mock::Mock

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
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): non-owner mark should return a permission error.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should rollback uploaded state.
// touch_last_active(touch_last_active)(positive): existing user should be touched successfully.
// touch_last_active(touch_last_active)(negative): missing user should rollback the transaction.
// delete(delete)(positive): owner delete should remove user, credentials, and memberships, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a user without an uploaded avatar should not enqueue prom records.
// delete(delete)(negative): non-owner delete should return a permission error without mutation.
// delete(delete)(negative): missing user should rollback state.

use super::*;

use crate::part::effect::event::Event;
use crate::part::prom::Payload;
use crate::part::prom::intention::{ImageIntention, ImageKind};
use crate::part_impl::prom_mock::MockPromRecord;
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::model::member::MemberInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::test_util::assert_expected_variant;

/// Builds a [`UserInfo`] fixture with default timestamps and no avatar.
pub(crate) fn user(id: &str, qid: &str, nickname: &str) -> UserInfo {
    let time = OffsetDateTime::now_utc();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

/// Builds a [`UserInfo`] fixture with avatar fields set.
pub(crate) fn user_with_avatar(
    id: &str,
    qid: &str,
    nickname: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: i64,
) -> UserInfo {
    UserInfo {
        avatar_key: Some(avatar_key.into()),
        avatar_uploaded,
        avatar_version,
        ..user(id, qid, nickname)
    }
}

/// Builds a [`UserCredential`] with a properly hashed password.
pub(crate) fn credential(user_id: &str, password: &str) -> UserCredential {
    let password_hash = match UserComplex::hash_password(password) {
        Ok(password_hash) => password_hash,
        Err(_) => panic!("failed to hash password"),
    };

    UserCredential {
        user_id: user_id.into(),
        password_hash,
    }
}

/// Builds a [`UserCredential`] that will never match any real password.
pub(crate) fn invalid_credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "invalid-password-hash".into(),
    }
}

/// Builds a [`MemberInfo`] fixture.
pub(crate) fn member(id: &str, user_id: &str, user_nickname: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_nickname.into(),
        team_id: team_id.into(),
        role_mask: crate::model::role::RoleMask::from(crate::model::role::RoleBit::ADMIN),
    }
}

/// Builds a [`UserToken`] fixture for the given user ID.
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

/// Builds an [`UpdateUserInfoData`] fixture.
fn update_data(id: &str, qid: &str, nickname: &str) -> UpdateUserInfoData {
    UpdateUserInfoData {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
    }
}

/// Builds a [`ReserveUserAvatarData`] fixture.
fn reserve_data(file_ext: &str) -> ReserveUserAvatarData {
    ReserveUserAvatarData {
        file_ext: file_ext.into(),
    }
}

/// Builds a [`MarkUserAvatarUploadedData`] fixture.
fn mark_data(avatar_version: i64) -> MarkUserAvatarUploadedData {
    MarkUserAvatarUploadedData { avatar_version }
}

/// Counts [`Delete`](ImageIntention::Delete) prom records matching the given object key.
fn count_delete_records(records: &[MockPromRecord], object_key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                Payload::Image(ImageIntention::Delete { object_key: key })
                    if key == object_key
            )
        })
        .count()
}

/// Counts [`CheckUploaded`](ImageIntention::CheckUploaded) prom records for user avatars.
fn count_user_check_records(
    records: &[MockPromRecord],
    resource_id: &str,
    object_key: &str,
    image_version: i64,
) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                &record.payload,
                Payload::Image(ImageIntention::CheckUploaded {
                    kind: ImageKind::UserAvatar,
                    resource_id: id,
                    object_key: key,
                    image_version: version,
                }) if id == resource_id && key == object_key && *version == image_version
            )
        })
        .count()
}

#[tokio::test]
async fn get_info_emits_active_for_self() {
    let mock = Mock::new();
    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let result = get_info(&mock, &mock, &mock, token("user-1"), "user-1".into()).await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.id, "user-1");
    assert_eq!(result.nickname, "Nick");
    let events = mock.drain_events();
    assert_eq!(events.len(), 1);
    let Event::UserActive(payload) = &events[0] else {
        panic!("expected UserActive event");
    };
    assert_eq!(payload.user_id, "user-1");
}

#[tokio::test]
async fn get_info_does_not_emit_active_for_other_user() {
    let mock = Mock::new();
    mock.seed_user(
        user("user-2", "qid-2", "Other"),
        credential("user-2", "password"),
    );

    let result = get_info(&mock, &mock, &mock, token("user-1"), "user-2".into()).await;
    assert!(result.is_ok());

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn get_info_propagates_missing_user() {
    let mock = Mock::new();

    let err = get_info(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn update_info_updates_user_and_member_nickname() {
    let mock = Mock::new();
    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "password"),
    );
    mock.seed_member(member("member-1", "user-1", "Old", "team-1"));

    let result = update_info(
        &mock,
        &mock,
        token("user-1"),
        update_data("user-1", "qid-new", "New"),
    )
    .await;
    assert!(result.is_ok());

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.users[0].qid, "qid-new");
    assert_eq!(snapshot.users[0].nickname, "New");
    assert_eq!(snapshot.members[0].user_nickname, "New");
}

#[tokio::test]
async fn update_info_rejects_non_owner_without_mutation() {
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
    let mock = Mock::new();
    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let result = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("png"),
    )
    .await;
    assert!(result.is_ok());
    let result = result.ok().unwrap();

    assert_eq!(result.avatar_version, 1);
    assert_eq!(
        result.put_url,
        "https://test.local/put/user_avatar/user-1-1.png"
    );

    let snapshot = mock.snapshot();
    assert_eq!(
        snapshot.users[0].avatar_key.as_deref(),
        Some("user_avatar/user-1-1.png")
    );
    assert!(!snapshot.users[0].avatar_uploaded);
    assert_eq!(
        count_user_check_records(
            &snapshot.prom_records,
            "user-1",
            "user_avatar/user-1-1.png",
            1
        ),
        1
    );
}

#[tokio::test]
async fn reserve_avatar_replacing_avatar_enqueues_delete_and_check() {
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "old-key", true, 1),
        credential("user-1", "password"),
    );

    let result = reserve_avatar(
        &mock,
        &mock,
        &mock,
        &mock,
        token("user-1"),
        reserve_data("jpg"),
    )
    .await;
    assert!(result.is_ok());

    let snapshot = mock.snapshot();
    assert_eq!(count_delete_records(&snapshot.prom_records, "old-key"), 1);
    assert_eq!(
        count_user_check_records(
            &snapshot.prom_records,
            "user-1",
            "user_avatar/user-1-2.jpg",
            2
        ),
        1
    );
}

#[tokio::test]
async fn reserve_avatar_rolls_back_missing_user() {
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
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let result =
        mark_avatar_uploaded(&mock, &mock, token("user-1"), "user-1".into(), mark_data(2)).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_non_owner() {
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let err = mark_avatar_uploaded(&mock, &mock, token("user-2"), "user-1".into(), mark_data(2))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert!(!mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn mark_avatar_uploaded_rolls_back_stale_version() {
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "key", false, 2),
        credential("user-1", "password"),
    );

    let err = mark_avatar_uploaded(&mock, &mock, token("user-1"), "user-1".into(), mark_data(1))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(!mock.snapshot().users[0].avatar_uploaded);
}

#[tokio::test]
async fn touch_last_active_succeeds_for_existing_user() {
    let mock = Mock::new();
    mock.seed_user(
        user("user-1", "qid-1", "Nick"),
        credential("user-1", "password"),
    );

    let result = touch_last_active(&mock, &mock, token("user-1")).await;

    assert!(result.is_ok());
}

#[tokio::test]
async fn touch_last_active_rolls_back_missing_user() {
    let mock = Mock::new();

    let err = touch_last_active(&mock, &mock, token("user-1"))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().users.is_empty());
}

#[tokio::test]
async fn delete_removes_user_credentials_members_and_enqueues_avatar_delete() {
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", true, 2),
        credential("user-1", "password"),
    );
    mock.seed_member(member("member-1", "user-1", "Nick", "team-1"));
    mock.seed_member(member("member-2", "user-1", "Nick", "team-2"));

    let result = delete(&mock, &mock, &mock, token("user-1"), "user-1".into()).await;
    assert!(result.is_ok());

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
    let mock = Mock::new();
    mock.seed_user(
        user_with_avatar("user-1", "qid-1", "Nick", "avatar-key", false, 2),
        credential("user-1", "password"),
    );

    let result = delete(&mock, &mock, &mock, token("user-1"), "user-1".into()).await;
    assert!(result.is_ok());

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_rejects_non_owner_without_mutation() {
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
    let mock = Mock::new();

    let err = delete(&mock, &mock, &mock, token("user-1"), "user-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().users.is_empty());
}
