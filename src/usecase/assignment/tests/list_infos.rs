// list_infos(list_infos)(positive): team member should list chapter assignments.
// list_infos(list_infos)(positive): assignment fallback should list chapter assignments.
// list_infos(list_infos)(positive): owner should list own assignments.
// list_infos(list_infos)(positive): super admin should list another user's assignments.
// list_infos(list_infos)(negative): unrelated user should be rejected from chapter assignments.
// list_infos(list_infos)(negative): non-owner non-admin should be rejected from user assignments.
// list_infos(list_infos)(negative): invalid owner combination should be rejected.

use super::*;

use crate::data::assignment::ListAssignmentInfosParams;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::assignment::AssignmentInclOpt;

#[tokio::test]
async fn list_infos_team_member_lists_chapter_assignments() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_user(&mock, "member-user", false);

    mock.seed_member(member("member-user", role(RoleField::TRANSLATOR)));

    mock.seed_assignment(assignment(
        "chapter-1",
        "member-user",
        role(RoleField::TRANSLATOR),
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "reviewer-user",
        role(RoleField::REVIEWER),
    ));

    let assignment_info_vals = list_infos(
        &mock,
        &mock,
        token("member-user"),
        list_by_chapter_data("chapter-1"),
    )
    .await;

    assert!(assignment_info_vals.is_ok());

    let assignment_info_vals = assignment_info_vals.ok().unwrap();

    assert_eq!(assignment_info_vals.len(), 2);
}

#[tokio::test]
async fn list_infos_deep_chapter_comic_workset_team_incl_fills_full_chain() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_user(&mock, "member-user", false);

    mock.seed_member(member("member-user", role(RoleField::TRANSLATOR)));

    mock.seed_assignment(assignment(
        "chapter-1",
        "member-user",
        role(RoleField::TRANSLATOR),
    ));

    let mut list_data = list_by_chapter_data("chapter-1");

    list_data.incl_opt = vec![AssignmentInclOpt::ChapterComicWorksetTeam];

    let assignment_info_vals =
        list_infos(&mock, &mock, token("member-user"), list_data)
            .await
            .ok()
            .unwrap();

    let chapter_info_val = assignment_info_vals[0].chapter.as_ref().unwrap();

    let comic_info_val = chapter_info_val.comic.as_ref().unwrap();

    assert_eq!(chapter_info_val.id, "chapter-1");

    assert_eq!(comic_info_val.id, "comic-1");

    assert_eq!(comic_info_val.workset.as_ref().unwrap().id, "workset-1");

    assert_eq!(comic_info_val.team.as_ref().unwrap().id, "team-1");
}

#[tokio::test]
async fn list_infos_assignment_fallback_lists_chapter_assignments() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_user(&mock, "assigned-user", false);

    mock.seed_assignment(assignment(
        "chapter-1",
        "assigned-user",
        role(RoleField::TRANSLATOR),
    ));

    let assignment_info_vals = list_infos(
        &mock,
        &mock,
        token("assigned-user"),
        list_by_chapter_data("chapter-1"),
    )
    .await;

    assert!(assignment_info_vals.is_ok());

    assert_eq!(assignment_info_vals.ok().unwrap().len(), 1);
}

#[tokio::test]
async fn list_infos_owner_lists_own_assignments() {
    //
    let mock = Mock::new();

    seed_user(&mock, "owner-user", false);

    mock.seed_assignment(assignment(
        "chapter-1",
        "owner-user",
        role(RoleField::TRANSLATOR),
    ));

    mock.seed_assignment(assignment(
        "chapter-2",
        "other-user",
        role(RoleField::REVIEWER),
    ));

    let assignment_info_vals = list_infos(
        &mock,
        &mock,
        token("owner-user"),
        list_by_user_data("owner-user"),
    )
    .await;

    assert!(assignment_info_vals.is_ok());

    let assignment_info_vals = assignment_info_vals.ok().unwrap();

    assert_eq!(assignment_info_vals.len(), 1);

    assert_eq!(assignment_info_vals[0].user_id, "owner-user");
}

#[tokio::test]
async fn list_infos_super_admin_lists_other_user_assignments() {
    //
    let mock = Mock::new();

    seed_user(&mock, "sadmin-user", true);

    seed_user(&mock, "target-user", false);

    mock.seed_assignment(assignment(
        "chapter-1",
        "target-user",
        role(RoleField::TRANSLATOR),
    ));

    let assignment_info_vals = list_infos(
        &mock,
        &mock,
        token("sadmin-user"),
        list_by_user_data("target-user"),
    )
    .await;

    assert!(assignment_info_vals.is_ok());

    assert_eq!(assignment_info_vals.ok().unwrap().len(), 1);
}

#[tokio::test]
async fn list_infos_unrelated_user_is_rejected_from_chapter_assignments() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    seed_user(&mock, "outsider-user", false);

    let err = list_infos(
        &mock,
        &mock,
        token("outsider-user"),
        list_by_chapter_data("chapter-1"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_non_owner_non_admin_is_rejected_from_user_assignments() {
    //
    let mock = Mock::new();

    seed_user(&mock, "viewer-user", false);

    seed_user(&mock, "target-user", false);

    let err = list_infos(
        &mock,
        &mock,
        token("viewer-user"),
        list_by_user_data("target-user"),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Perm);
}

#[tokio::test]
async fn list_infos_invalid_owner_combination_is_rejected() {
    //
    let mock = Mock::new();

    seed_user(&mock, "viewer-user", false);

    let err = list_infos(
        &mock,
        &mock,
        token("viewer-user"),
        ListAssignmentInfosParams {
            incl_opt: Vec::new(),
            chapter_id: Some("chapter-1".into()),
            owner_id: Some("owner-user".into()),
            role: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}
