#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// list_infos(list_infos)(positive): team member should list team announcements.
// list_infos(list_infos)(positive): user include should be populated only when requested.
// list_infos(list_infos)(negative): non-member should be rejected from team announcements.
// create(create)(positive): team admin should create an announcement.
// create(create)(positive): announcement text should be trimmed before persistence.
// create(create)(negative): non-admin member should be rejected without mutation.
// create(create)(negative): non-member should be rejected without mutation.
// update_info(update_info)(positive): team admin should replace announcement content.
// update_info(update_info)(negative): non-admin member should be rejected without mutation.
// update_info(update_info)(negative): missing announcement should be rejected without mutation.
// delete(delete)(positive): team admin should delete an announcement.
// delete(delete)(negative): non-admin member should be rejected without mutation.
// delete(delete)(negative): missing announcement should be rejected without mutation.

use super::*;

use time::OffsetDateTime;

use crate::data::instr::announcement::{
    CreateAnnouncementInstr, ListAnnouncementInfosInstr,
    UpdateAnnouncementInfoInstr,
};
use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{assert_expected_variant, now};
use crate::value::announcement::AnnouncementInclOpt;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    // Build a user token fixture for announcement perm checks.
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    // Build a user credential fixture for announcement-related authentication flows.
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, nickname: &str) -> UserInfo {
    // Build a visible team user with deterministic timestamps.
    //
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: nickname.into(),
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn member(
    id: &str,
    user_id: &str,
    team_id: &str,
    role_mask: RoleMask,
) -> MemberInfo {
    MemberInfo {
        id: id.into(),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: now(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn announcement(
    id: &str,
    team_id: &str,
    user_id: &str,
    created_at: OffsetDateTime,
) -> AnnouncementInfo {
    AnnouncementInfo {
        id: id.into(),
        team_id: team_id.into(),
        user_id: user_id.into(),
        user: None,
        title: "title".into(),
        content: "content".into(),
        created_at,
    }
}

fn list_instr(
    team_id: &str,
    incl_opt: Vec<AnnouncementInclOpt>,
) -> ListAnnouncementInfosInstr {
    ListAnnouncementInfosInstr {
        team_id: team_id.into(),
        incl_opt,
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    }
}

fn create_instr(team_id: &str) -> CreateAnnouncementInstr {
    CreateAnnouncementInstr {
        team_id: team_id.into(),
        title: "title".into(),
        content: "created".into(),
    }
}

fn update_instr(id: &str) -> UpdateAnnouncementInfoInstr {
    UpdateAnnouncementInfoInstr {
        id: id.into(),
        title: "updated title".into(),
        content: "updated content".into(),
    }
}

fn seed_member(mock: &Mock, user_id: &str, team_id: &str, role_mask: RoleMask) {
    mock.seed_member(member(
        "member--00000000001",
        user_id,
        team_id,
        role_mask,
    ));
}

#[tokio::test]
async fn list_infos_team_member_lists_team_announcements() {
    //
    let mock = Mock::new();

    let time = now();

    seed_member(
        &mock,
        "viewer-user",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        time,
    ));

    mock.seed_announcement(announcement(
        "announcement--00000000002",
        "team--00000000002",
        "author-user",
        time,
    ));

    let announcement_info_vals = list_infos(
        (&mock, &mock),
        token("viewer-user"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await
    .unwrap();

    assert_eq!(announcement_info_vals.len(), 1);

    assert_eq!(
        announcement_info_vals.first().unwrap().id,
        "announcement--00000000001"
    );
}

#[tokio::test]
async fn list_infos_user_include_follows_request() {
    //
    let mock = Mock::new();

    let time = now();

    seed_member(
        &mock,
        "viewer-user",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    mock.seed_user(user("author-user", "Author"), credential("author-user"));

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        time,
    ));

    let without_user = list_infos(
        (&mock, &mock),
        token("viewer-user"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await
    .unwrap();

    assert!(without_user.first().unwrap().user.is_none());

    let with_user = list_infos(
        (&mock, &mock),
        token("viewer-user"),
        list_instr("team--00000000001", vec![AnnouncementInclOpt::User]),
    )
    .await
    .unwrap();

    assert_eq!(
        with_user.first().unwrap().user.as_ref().unwrap().id,
        "author-user"
    );
}

#[tokio::test]
async fn list_infos_non_member_is_rejected() {
    //
    let mock = Mock::new();

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        now(),
    ));

    let error = list_infos(
        (&mock, &mock),
        token("outsider-user"),
        list_instr("team--00000000001", Vec::new()),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_team_admin_creates_announcement() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    let created_announcement = create(
        &mock,
        token("admin-user"),
        create_instr("team--00000000001"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.announcements.len(), 1);

    assert_eq!(
        snapshot.announcements.first().unwrap().id,
        created_announcement.id
    );

    assert_eq!(
        snapshot.announcements.first().unwrap().team_id,
        "team--00000000001"
    );

    assert_eq!(
        snapshot.announcements.first().unwrap().user_id,
        "admin-user"
    );
}

#[tokio::test]
async fn create_and_update_trim_unicode_whitespace_from_announcement_texts() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    let mut create_instr = create_instr("team--00000000001");

    create_instr.title = "\u{2003}created title\u{2003}".into();

    create_instr.content = "\u{2003}created content\u{2003}".into();

    create(&mock, token("admin-user"), create_instr)
        .await
        .unwrap();

    let created = mock.snapshot().announcements.first().unwrap().clone();

    assert_eq!(created.title, "created title");

    assert_eq!(created.content, "created content");

    let mut update_instr = update_instr(&created.id);

    update_instr.title = "\u{2003}updated title\u{2003}".into();

    update_instr.content = "\u{2003}updated content\u{2003}".into();

    update_info(&mock, token("admin-user"), update_instr)
        .await
        .unwrap();

    let updated = mock.snapshot().announcements.first().unwrap().clone();

    assert_eq!(updated.title, "updated title");

    assert_eq!(updated.content, "updated content");
}

#[tokio::test]
async fn create_non_admin_member_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "member--66614769577",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    let error = create(
        &mock,
        token("member--66614769577"),
        create_instr("team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert!(mock.snapshot().announcements.is_empty());
}

#[tokio::test]
async fn create_non_member_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    let error = create(
        &mock,
        token("outsider-user"),
        create_instr("team--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert!(mock.snapshot().announcements.is_empty());
}

#[tokio::test]
async fn update_info_team_admin_replaces_announcement_content() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        now(),
    ));

    update_info(
        &mock,
        token("admin-user"),
        update_instr("announcement--00000000001"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(
        snapshot.announcements.first().unwrap().title,
        "updated title"
    );

    assert_eq!(
        snapshot.announcements.first().unwrap().content,
        "updated content"
    );

    assert_eq!(
        snapshot.announcements.first().unwrap().user_id,
        "author-user"
    );
}

#[tokio::test]
async fn update_info_non_admin_member_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "member--66614769577",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        now(),
    ));

    let error = update_info(
        &mock,
        token("member--66614769577"),
        update_instr("announcement--00000000001"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert_eq!(
        mock.snapshot().announcements.first().unwrap().title,
        "title"
    );
}

#[tokio::test]
async fn update_info_missing_announcement_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    let error =
        update_info(&mock, token("admin-user"), update_instr("missing"))
            .await
            .err()
            .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(mock.snapshot().announcements.is_empty());
}

#[tokio::test]
async fn delete_team_admin_deletes_announcement() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        now(),
    ));

    delete(
        &mock,
        token("admin-user"),
        "announcement--00000000001".into(),
    )
    .await
    .unwrap();

    assert!(mock.snapshot().announcements.is_empty());
}

#[tokio::test]
async fn delete_non_admin_member_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "member--66614769577",
        "team--00000000001",
        RoleMask::from(RoleField::TRANSLATOR),
    );

    mock.seed_announcement(announcement(
        "announcement--00000000001",
        "team--00000000001",
        "author-user",
        now(),
    ));

    let error = delete(
        &mock,
        token("member--66614769577"),
        "announcement--00000000001".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Perm);

    assert_eq!(mock.snapshot().announcements.len(), 1);
}

#[tokio::test]
async fn delete_missing_announcement_is_rejected_without_mutation() {
    //
    let mock = Mock::new();

    seed_member(
        &mock,
        "admin-user",
        "team--00000000001",
        RoleMask::from(RoleField::ADMIN),
    );

    let error = delete(&mock, token("admin-user"), "missing".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    assert!(mock.snapshot().announcements.is_empty());
}
