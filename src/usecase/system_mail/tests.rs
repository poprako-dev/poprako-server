//! Test fixtures and cases for the system mail use case module.
//!
//! Tests exercise listing unread mails and marking as read against
//! a [`Mock`] that doubles as the repository.
//!
//! [`Mock`]: crate::part_impl::repo::mock_impl::Mock

// list(list)(positive): should return only current user's mails matching the read filter, excluding other users.
// list(list)(positive): should apply pagination after sorting by created_at descending.
// list(list)(positive): offset exceeding result count should return an empty vec.
// mark_read(mark_read)(positive): should mark a batch of mails as read after verifying ownership.
// mark_read(mark_read)(negative): a nonexistent id in the batch should short-circuit with an argument error.
// mark_read(mark_read)(negative): a mail belonging to another user should return a permission error without mutation.

use super::*;

use time::{Duration, OffsetDateTime};

use crate::data::system_mail_data;
use crate::model::{system_mail_model, user_model};
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;

/// Builds a [`SystemMailInfo`] fixture.
fn mail(
    id: &str,
    receiver_id: &str,
    read: bool,
    created_at: OffsetDateTime,
) -> system_mail_model::Info {
    system_mail_model::Info {
        id: id.into(),
        receiver_id: receiver_id.into(),
        read,
        title: "title".into(),
        content: "content".into(),
        created_at,
    }
}

/// Builds a [`UserToken`] fixture.
fn token(user_id: &str) -> user_model::Token {
    user_model::Token {
        user_id: user_id.into(),
    }
}

/// Builds a [`ListSystemMailData`] for listing unread mails.
fn list_unread_data(offset: u64, limit: u64) -> system_mail_data::ListData {
    system_mail_data::ListData {
        read: Some(false),
        offset,
        limit,
    }
}

#[tokio::test]
async fn list_returns_current_user_unread_mails() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, time));

    mock.seed_system_mail(mail("sys_mail-2", "user-1", true, time)); // already read

    mock.seed_system_mail(mail("sys_mail-3", "user-2", false, time)); // other user

    let mails = list_infos(&mock, token("user-1"), list_unread_data(0, 10))
        .await
        .unwrap();

    assert_eq!(mails.len(), 1);

    assert_eq!(mails[0].id, "sys_mail-1");
}

#[tokio::test]
async fn list_applies_pagination_after_desc_sort() {
    //
    let mock = Mock::new();

    let t1 = OffsetDateTime::now_utc();

    let t2 = t1 + Duration::seconds(10);

    let t3 = t2 + Duration::seconds(10);

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, t1));

    mock.seed_system_mail(mail("sys_mail-2", "user-1", false, t3));

    mock.seed_system_mail(mail("sys_mail-3", "user-1", false, t2));

    let mails = list_infos(&mock, token("user-1"), list_unread_data(0, 2))
        .await
        .unwrap();

    assert_eq!(mails.len(), 2);

    // Should be sorted by created_at DESC.
    assert_eq!(mails[0].id, "sys_mail-2");

    assert_eq!(mails[1].id, "sys_mail-3");
}

#[tokio::test]
async fn list_returns_empty_for_missing_page() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, time));

    let mails = list_infos(&mock, token("user-1"), list_unread_data(10, 10))
        .await
        .unwrap();

    assert!(mails.is_empty());
}

#[tokio::test]
async fn mark_read_marks_batch_of_mails() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, time));

    mock.seed_system_mail(mail("sys_mail-2", "user-1", false, time));

    mark_read(
        &mock,
        token("user-1"),
        vec!["sys_mail-1".into(), "sys_mail-2".into()],
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert!(snapshot.system_mails[0].read);

    assert!(snapshot.system_mails[1].read);
}

#[tokio::test]
async fn mark_read_short_circuits_on_missing_id() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, time));

    let err = mark_read(
        &mock,
        token("user-1"),
        vec!["sys_mail-1".into(), "sys_mail-nonexistent".into()],
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    // First ID was marked before the second failed (non-transactional).
    let snapshot = mock.snapshot();

    assert!(snapshot.system_mails[0].read);
}

#[tokio::test]
async fn mark_read_rejects_other_user_mail() {
    //
    let mock = Mock::new();

    let time = OffsetDateTime::now_utc();

    mock.seed_system_mail(mail("sys_mail-1", "user-1", false, time));

    let err = mark_read(&mock, token("user-2"), vec!["sys_mail-1".into()])
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    let snapshot = mock.snapshot();

    assert!(!snapshot.system_mails[0].read);
}
