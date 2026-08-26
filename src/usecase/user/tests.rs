//! Test fixtures and cases for the user use case module.
//!
//! Tests exercise user profile reads, updates, avatar management, activity
//! tracking, and account deletion against a [`Mock`] that doubles as the
//! coordinator, repository, prom enqueuer, image pool, and effect developer.
//!
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

mod extra;
// Delete flow coverage, including avatar cleanup and related records.
mod delete;

// get_info(get_info)(positive): a user reading itself should receive info and emit UserActive.
// get_info(get_info)(positive): reading another user should not emit UserActive.
// get_info(get_info)(negative): missing user should propagate an argument error.
// update_info(update_info)(positive): owner update should change user info and member nickname.
// update_info(update_info)(negative): non-owner update should return a perm error without mutation.
// update_info(update_info)(negative): missing user should rollback the transaction.
// reserve_avatar(reserve_avatar)(positive): first reservation should update avatar state, enqueue a check, and return a put URL.
// reserve_avatar(reserve_avatar)(positive): replacing an avatar should enqueue delete and check messages.
// reserve_avatar(reserve_avatar)(negative): missing user should rollback avatar and prom state.
// reserve_avatar(reserve_avatar)(negative): put URL failure should propagate after transaction commit.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): matching owner and version should mark the avatar uploaded.
// mark_avatar_uploaded(mark_avatar_uploaded)(positive): repeated matching version confirmation should remain successful.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): non-owner mark should return a perm error.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): stale version should rollback uploaded state.
// mark_avatar_uploaded(mark_avatar_uploaded)(negative): old reservation replay should fail without marking current avatar uploaded.
// delete(delete)(positive): owner delete should remove user, credentials, and memberships, and enqueue uploaded avatar deletion.
// delete(delete)(positive): deleting a user without an uploaded avatar should not enqueue prom records.
// delete(delete)(negative): deleting a user must retain an admin in every affected team.
// delete(delete)(negative): non-owner delete should return a perm error without mutation.
// delete(delete)(negative): missing user should rollback state.

use super::*;

use time::OffsetDateTime;

use crate::complex::user::UserComplex;
use crate::data::instr::user::{
    MarkUserAvatarUploadedInstr, ReserveUserAvatarInstr, UpdateUserInfoInstr,
    UpdateUserPasswordInstr,
};
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::shared::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::image::ImagePayload;
use crate::part_impl::prom::mock_impl::MockPromRecord;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::fixture::{credential, user};
use crate::test_util::{
    IMAGE_CONFIG, assert_expected_message, assert_expected_variant,
    assert_one_image_check_record,
};
use crate::usecase::user::delete::delete;
use crate::value::image::{ImageExt, ImageHash, ImageKind};
use crate::value::role::{RoleField, RoleMask};

/// Builds a [`UserInfo`] fixture with avatar fields set.
fn user_with_avatar(
    id: &str,
    qid: &str,
    nickname: &str,
    avatar_key: &str,
    avatar_uploaded: bool,
    avatar_version: u32,
) -> UserInfo {
    UserInfo {
        avatar_key: Some(avatar_key.into()),
        is_avatar_uploaded: Some(avatar_uploaded),
        avatar_version: Some(avatar_version),
        ..user(id, qid, nickname)
    }
}

/// Builds a [`MemberInfo`] fixture.
fn member(
    id: &str,
    user_id: &str,
    user_nickname: &str,
    team_id: &str,
) -> MemberInfo {
    MemberInfo {
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
fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

/// Builds an [`UpdateUserInfoData`] fixture.
fn update_instr(id: &str, qid: &str, nickname: &str) -> UpdateUserInfoInstr {
    UpdateUserInfoInstr {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
    }
}

/// Builds [`UpdateUserPasswordInstr`] for replacing a user's password.
fn update_password_instr(
    current_password: &str,
    new_password: &str,
) -> UpdateUserPasswordInstr {
    UpdateUserPasswordInstr {
        current_password: current_password.into(),
        new_password: new_password.into(),
    }
}

/// Builds a [`ReserveUserAvatarData`] fixture.
fn reserve_instr(file_ext: &str) -> ReserveUserAvatarInstr {
    ReserveUserAvatarInstr {
        image_hash: ImageHash::new([1; 32]),
        new_byte_len: 4096,
        ext: ImageExt::parse(file_ext).unwrap(),
    }
}

/// Builds a [`MarkUserAvatarUploadedData`] fixture.
fn mark_instr(image_version: u32) -> MarkUserAvatarUploadedInstr {
    MarkUserAvatarUploadedInstr { image_version }
}

/// Counts deferred image-delete records matching the given object key.
fn count_delete_records(records: &[MockPromRecord], object_key: &str) -> usize {
    records
        .iter()
        .filter(|record| {
            matches!(
                record.payload(),
                TaskPayload::Image {
                    payload: ImagePayload::Delete { object_key: key },
                }
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

    let val = get_info((&mock, &mock, &mock), token("user-1"), "user-1".into())
        .await
        .unwrap();

    assert_eq!(val.id, "user-1");

    assert_eq!(val.nickname, "Nick");

    let events = mock.drain_events();

    assert_eq!(events.len(), 1);

    let Event::UserActive { payload } = &events[0] else {
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

    get_info((&mock, &mock, &mock), token("user-1"), "user-2".into())
        .await
        .unwrap();

    assert_eq!(mock.event_count(), 0);
}

#[tokio::test]
async fn get_info_propagates_missing_user() {
    //
    let mock = Mock::new();

    let err = get_info((&mock, &mock, &mock), token("user-1"), "user-1".into())
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
        (&mock, &mock),
        token("user-1"),
        update_instr("user-1", "qid-new", "New"),
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
        (&mock, &mock),
        token("user-2"),
        update_instr("user-1", "qid-new", "New"),
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
        (&mock, &mock),
        token("user-1"),
        update_instr("user-1", "qid-new", "New"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().users.is_empty());
}

#[tokio::test]
async fn update_password_replaces_the_verified_password() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "old-password"),
    );

    update_password(
        (&mock, &mock),
        token("user-1"),
        "user-1".into(),
        update_password_instr("old-password", "new-password"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(
        UserComplex::verify_password(
            "new-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );

    assert!(
        !UserComplex::verify_password(
            "old-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );
}

#[tokio::test]
async fn update_password_rejects_an_incorrect_current_password() {
    //
    let mock = Mock::new();

    mock.seed_user(
        user("user-1", "qid-1", "Old"),
        credential("user-1", "old-password"),
    );

    let err = update_password(
        (&mock, &mock),
        token("user-1"),
        "user-1".into(),
        update_password_instr("wrong-password", "new-password"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Auth);

    let snapshot = mock.snapshot();

    assert!(
        UserComplex::verify_password(
            "old-password",
            &snapshot.credentials[0].password_hash,
        )
        .await
    );
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
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        reserve_instr("png"),
    )
    .await
    .unwrap();

    assert_eq!(val.slot.as_ref().unwrap().image_version, 1);

    assert_eq!(
        val.slot.as_ref().unwrap().put_url,
        "https://test.local/put/user_avatar/user-1-1.png"
    );

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.users[0].avatar_key.as_deref(),
        Some("user_avatar/user-1-1.png")
    );

    assert_ne!(snapshot.users[0].is_avatar_uploaded, Some(true));

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
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        reserve_instr("jpg"),
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
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        reserve_instr("png"),
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
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        reserve_instr("png"),
    )
    .await
    .err()
    .unwrap();

    assert!(matches!(err, BaseError::Unrecoverable { .. }));

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
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
        mark_instr(2),
    )
    .await
    .unwrap();

    assert_eq!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
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
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
        mark_instr(2),
    )
    .await;

    assert!(first.is_ok());

    let second = mark_avatar_uploaded(
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
        mark_instr(2),
    )
    .await;

    assert!(second.is_ok());

    assert_eq!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
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
        (&mock, &mock, &mock),
        token("user-2"),
        "user-1".into(),
        mark_instr(2),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_ne!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
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
        (&mock, &mock, &mock),
        token("user-1"),
        "user-1".into(),
        mark_instr(1),
    )
    .await
    .err()
    .unwrap();

    assert_expected_message(
        err,
        ExpectedVariant::Args,
        "error-stale-avatar-upload",
    );

    assert_ne!(mock.snapshot().users[0].is_avatar_uploaded, Some(true));
}
