// list_infos(list_infos)(positive): reviewer should list chapter invitations.
// list_infos(list_infos)(negative): non-reviewer should be rejected.
// create(create)(positive): reviewer should create a pending assignment invitation.
// create(create)(negative): invitee with existing assignment should be rejected.
// delete(delete)(positive): reviewer should delete an invitation.
// delete(delete)(negative): non-reviewer should be rejected without deleting invitation.
// join(join)(positive): invited user should create assignment and consume invitation.
// join(join)(positive): invited user should merge roles into existing assignment.
// join(join)(negative): mismatched user qid should be rejected without consuming invitation.

use super::*;

use crate::model::assignment::AssignmentInfo;
use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo};
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{assert_expected_variant, now};
use crate::value::chapter::StageMask;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn user(id: &str, qid: &str, nickname: &str) -> UserInfo {
    let time = now();

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

fn team(id: &str) -> TeamInfo {
    let time = now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        workset_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    let time = now();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: id.into(),
        description: None,
        comic_count: 0,
        comic_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    let time = now();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: id.into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
        creator_id: "creator-user".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str) -> ChapterInfo {
    let time = now();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "subtitle".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "creator-user".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: now(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn assignment(chapter_id: &str, user_id: &str, role_mask: RoleMask) -> AssignmentInfo {
    let time = now();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn invitation(id: &str, invitee_qid: &str, role_mask: RoleMask) -> AssignmentInvitationInfo {
    let time = now();

    AssignmentInvitationInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        inviter_id: "reviewer-user".into(),
        invitee_qid: invitee_qid.into(),
        code: "AINV123".into(),
        pending: true,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn role(role_field: RoleField) -> RoleMask {
    RoleMask::from(role_field)
}

fn list_data() -> ListAssignmentInvitationInfosData {
    ListAssignmentInvitationInfosData {
        chapter_id: "chapter-1".into(),
        pending: Some(true),
        offset: 0,
        limit: 10,
    }
}

fn create_data(invitee_qid: &str) -> CreateAssignmentInvitationData {
    CreateAssignmentInvitationData {
        chapter_id: "chapter-1".into(),
        invitee_qid: invitee_qid.into(),
        roles: role(RoleField::TRANSLATOR),
    }
}

fn join_data() -> JoinAssignmentInvitationData {
    JoinAssignmentInvitationData {
        code: "AINV123".into(),
    }
}

fn seed_scope(mock: &Mock) {
    mock.seed_team(team("team-1"));
    mock.seed_workset(workset("workset-1", "team-1"));
    mock.seed_comic(comic("comic-1", "workset-1"));
    mock.seed_chapter(chapter("chapter-1", "comic-1"));
}

fn seed_reviewer(mock: &Mock) {
    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        role(RoleField::REVIEWER),
    ));
}

#[tokio::test]
async fn list_infos_reviewer_lists_chapter_invitations() {
    let mock = Mock::new();
    seed_scope(&mock);
    seed_reviewer(&mock);
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let val = list_infos(&mock, token("reviewer-user"), list_data())
        .await
        .unwrap();

    assert_eq!(val.len(), 1);
    assert_eq!(val[0].id, "invitation-1");
}

#[tokio::test]
async fn list_infos_non_reviewer_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = list_infos(&mock, token("normal-user"), list_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_reviewer_creates_pending_invitation() {
    let mock = Mock::new();
    seed_scope(&mock);
    seed_reviewer(&mock);
    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );

    let val = create(
        &mock,
        &mock,
        token("reviewer-user"),
        create_data("target-qid"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.assignment_invitations.len(), 1);
    assert_eq!(snapshot.assignment_invitations[0].id, val.id);
    assert_eq!(snapshot.assignment_invitations[0].code, val.code);
    assert_eq!(snapshot.assignment_invitations[0].chapter_id, "chapter-1");
    assert_eq!(
        snapshot.assignment_invitations[0].inviter_id,
        "reviewer-user"
    );
    assert_eq!(snapshot.assignment_invitations[0].invitee_qid, "target-qid");
    assert!(snapshot.assignment_invitations[0].pending);
}

#[tokio::test]
async fn create_existing_assignment_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    seed_reviewer(&mock);
    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );
    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::TRANSLATOR),
    ));

    let err = create(
        &mock,
        &mock,
        token("reviewer-user"),
        create_data("target-qid"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().assignment_invitations.is_empty());
}

#[tokio::test]
async fn delete_reviewer_deletes_invitation() {
    let mock = Mock::new();
    seed_scope(&mock);
    seed_reviewer(&mock);
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    delete(&mock, &mock, token("reviewer-user"), "invitation-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().assignment_invitations.is_empty());
}

#[tokio::test]
async fn delete_non_reviewer_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = delete(&mock, &mock, token("normal-user"), "invitation-1".into())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
    assert_eq!(mock.snapshot().assignment_invitations.len(), 1);
}

#[tokio::test]
async fn join_invited_user_creates_assignment_and_consumes_invitation() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );
    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    join(&mock, &mock, &mock, token("target-user"), join_data())
        .await
        .unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.assignments.len(), 1);
    assert_eq!(snapshot.assignments[0].chapter_id, "chapter-1");
    assert_eq!(snapshot.assignments[0].user_id, "target-user");
    assert_eq!(snapshot.assignments[0].roles, role(RoleField::TRANSLATOR));
    assert!(!snapshot.assignment_invitations[0].pending);
}

#[tokio::test]
async fn join_existing_assignment_merges_roles() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );
    mock.seed_member(member(
        "target-user",
        role(RoleField::TRANSLATOR).union(role(RoleField::PROOFREADER)),
    ));
    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::TRANSLATOR),
    ));
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::PROOFREADER),
    ));

    join(&mock, &mock, &mock, token("target-user"), join_data())
        .await
        .unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.assignments.len(), 1);
    assert!(
        snapshot.assignments[0]
            .roles
            .has_every_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    );
    assert!(!snapshot.assignment_invitations[0].pending);
}

#[tokio::test]
async fn join_mismatched_qid_is_rejected() {
    let mock = Mock::new();
    seed_scope(&mock);
    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );
    mock.seed_member(member("target-user", role(RoleField::TRANSLATOR)));
    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "other-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = join(&mock, &mock, &mock, token("target-user"), join_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    let snapshot = mock.snapshot();
    assert!(snapshot.assignments.is_empty());
    assert!(snapshot.assignment_invitations[0].pending);
}
