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

use crate::data::assignment_invitation::{
    CreateAssignmentInvitationParams, JoinAssignmentInvitationParams,
    ListAssignmentInvitationInfosParams,
};
use crate::model::assignment::AssignmentInfo;
use crate::model::assignment_invitation::AssignmentInvitationInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo, UserToken};
use crate::model::workset::WorksetInfo;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{assert_expected_variant, now};
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

// Build a token fixture for invitation test requests.
fn token(user_id: &str) -> UserToken {
    // Build a caller token for assignment-invitation API tests.
    UserToken {
        user_id: user_id.into(),
    }
}

// Build a stable credential fixture for authentication setup.
fn credential(user_id: &str) -> UserCredential {
    // Build a credential fixture with fixed digest for login assertions.
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

// Build a user fixture used by invitation creator/invitee flows.
fn user(id: &str, qid: &str, nickname: &str) -> UserInfo {
    //
    // Build a minimal user fixture used by invitation creator/invitee paths.
    let time = now();

    UserInfo {
        id: id.into(),
        qid: qid.into(),
        nickname: nickname.into(),
        avatar_key: None,
        is_avatar_uploaded: false,
        avatar_version: 0,
        avatar_hash: ImageHash::default(),
        avatar_ext: ImageExt::Png,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

// Build a team fixture used as owner context for invitations.
fn team(id: &str) -> TeamInfo {
    //
    // Build a team fixture used as ownership anchor for invitations.
    let time = now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        avatar_key: None,
        is_avatar_uploaded: false,
        avatar_version: 0,
        avatar_hash: ImageHash::default(),
        avatar_ext: ImageExt::Png,
        created_at: time,
        updated_at: time,
    }
}

// Build a workset fixture with fixed team ownership.
fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    // Build a workset fixture shared by comic and chapter test entities.
    let time = now();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: id.into(),
        description: None,
        comic_count: 0,
        created_at: time,
        updated_at: time,
    }
}

// Build a comic fixture attached to given workset.
fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    // Build a comic fixture under the chosen workset.
    let time = now();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: id.into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        is_cover_uploaded: false,
        cover_version: 0,
        cover_hash: ImageHash::default(),
        cover_ext: ImageExt::Png,
        chapter_count: 1,
        creator_id: "creator-user".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

// Build a chapter fixture with an initialized stage mask.
fn chapter(id: &str, comic_id: &str) -> ChapterInfo {
    //
    // Build a chapter fixture for invitation lifecycle tests.
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

// Build a member fixture with deterministic role assignment.
fn member(user_id: &str, role_mask: RoleMask) -> MemberInfo {
    // Build a team member fixture with a stable role assignment.
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

// Build an assignment fixture for role-merge/duplication assertions.
fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    // Build an active assignment fixture for duplicate or merge-role assertions.
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

// Build a pending invitation fixture for list/create/join scenarios.
fn invitation(
    id: &str,
    invitee_qid: &str,
    role_mask: RoleMask,
) -> AssignmentInvitationInfo {
    //
    // Build an invitation fixture with deterministic code and role payload.
    let time = now();

    AssignmentInvitationInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        inviter_id: "admin-user".into(),
        invitee_qid: invitee_qid.into(),
        code: "AINV123".into(),
        is_pending: true,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

// Build a one-role mask for invitation payloads.
fn role(role_field: RoleField) -> RoleMask {
    // Build a single-role bitmask used by invitation payloads.
    RoleMask::from(role_field)
}

// Build list params that fetch current pending invitation list.
fn list_data() -> ListAssignmentInvitationInfosParams {
    ListAssignmentInvitationInfosParams {
        chapter_id: "chapter-1".into(),
        is_pending: Some(true),
        offset: 0,
        limit: 10,
    }
}

// Build create params targeting a specific invitee qid.
fn create_data(invitee_qid: &str) -> CreateAssignmentInvitationParams {
    CreateAssignmentInvitationParams {
        chapter_id: "chapter-1".into(),
        invitee_qid: invitee_qid.into(),
        roles: role(RoleField::TRANSLATOR),
    }
}

// Build join params with a deterministic invitation code.
fn join_data() -> JoinAssignmentInvitationParams {
    JoinAssignmentInvitationParams {
        code: "AINV123".into(),
    }
}

// Seed shared team/workset/comic/chapter references for all invitation tests.
fn seed_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1"));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_chapter(chapter("chapter-1", "comic-1"));
}

// Seed an admin assignment baseline used by invite/reject checks.
fn seed_admin(mock: &Mock) {
    mock.seed_assignment(assignment(
        "chapter-1",
        "admin-user",
        role(RoleField::ADMIN),
    ));
}

#[tokio::test]
async fn list_infos_reviewer_lists_chapter_invitations() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let val = list_infos((&mock,), token("admin-user"), list_data())
        .await
        .unwrap();

    assert_eq!(val.len(), 1);

    assert_eq!(val[0].id, "invitation-1");
}

#[tokio::test]
async fn list_infos_non_reviewer_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err = list_infos((&mock,), token("normal-user"), list_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn create_reviewer_creates_pending_invitation() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    mock.seed_user(
        user("target-user", "target-qid", "Target"),
        credential("target-user"),
    );

    let before = now();

    let val = create(
        (&mock, &mock, &mock),
        token("admin-user"),
        create_data("target-qid"),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignment_invitations.len(), 1);

    assert_eq!(snapshot.assignment_invitations[0].id, val.id);

    assert_eq!(snapshot.assignment_invitations[0].code, val.code);

    assert_eq!(snapshot.assignment_invitations[0].chapter_id, "chapter-1");

    assert_eq!(snapshot.assignment_invitations[0].inviter_id, "admin-user");

    assert_eq!(snapshot.assignment_invitations[0].invitee_qid, "target-qid");

    assert!(snapshot.assignment_invitations[0].is_pending);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(
        snapshot.prom_records[0].payload(),
        TaskPayload::Invitation(InvitationPayload::Assignment {
            invitation_id: val.id,
        })
    );

    assert!(snapshot.prom_records[0].visible_at() >= before + EXPIRY_DELAY);

    assert!(snapshot.prom_records[0].visible_at() <= now() + EXPIRY_DELAY);
}

#[tokio::test]
async fn create_rejects_published_chapter() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    {
        let mut state = mock.state.lock().unwrap();

        state.chapters[0].stages = state.chapters[0]
            .stages
            .try_set_phase(Stage::Publish, StagePhase::Completed)
            .unwrap();
    }

    let result = create(
        (&mock, &mock, &mock),
        token("admin-user"),
        create_data("target-qid"),
    )
    .await;

    assert!(matches!(result, Err(BaseError::Expected { .. })));

    assert!(mock.snapshot().assignment_invitations.is_empty());
}

#[tokio::test]
async fn create_existing_assignment_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

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
        (&mock, &mock, &mock),
        token("admin-user"),
        create_data("target-qid"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert!(mock.snapshot().assignment_invitations.is_empty());

    assert!(mock.snapshot().prom_records.is_empty());
}

#[tokio::test]
async fn delete_reviewer_deletes_invitation() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_admin(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    delete((&mock, &mock), token("admin-user"), "invitation-1".into())
        .await
        .unwrap();

    assert!(mock.snapshot().assignment_invitations.is_empty());
}

#[tokio::test]
async fn delete_non_reviewer_is_rejected() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_assignment_invitation(invitation(
        "invitation-1",
        "target-qid",
        role(RoleField::TRANSLATOR),
    ));

    let err =
        delete((&mock, &mock), token("normal-user"), "invitation-1".into())
            .await
            .err()
            .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);

    assert_eq!(mock.snapshot().assignment_invitations.len(), 1);
}

#[tokio::test]
async fn join_invited_user_creates_assignment_and_consumes_invitation() {
    //
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

    join((&mock, &mock, &mock), token("target-user"), join_data())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignments.len(), 1);

    assert_eq!(snapshot.assignments[0].chapter_id, "chapter-1");

    assert_eq!(snapshot.assignments[0].user_id, "target-user");

    assert_eq!(snapshot.assignments[0].roles, role(RoleField::TRANSLATOR));

    assert!(!snapshot.assignment_invitations[0].is_pending);
}

#[tokio::test]
async fn join_existing_assignment_merges_roles() {
    //
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

    join((&mock, &mock, &mock), token("target-user"), join_data())
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.assignments.len(), 1);

    assert!(
        snapshot.assignments[0]
            .roles
            .has_every_role(&[RoleField::TRANSLATOR, RoleField::PROOFREADER])
    );

    assert!(!snapshot.assignment_invitations[0].is_pending);
}

#[tokio::test]
async fn join_mismatched_qid_is_rejected() {
    //
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

    let err = join((&mock, &mock, &mock), token("target-user"), join_data())
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.assignments.is_empty());

    assert!(snapshot.assignment_invitations[0].is_pending);
}
