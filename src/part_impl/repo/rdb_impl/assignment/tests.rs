// assignment_roundtrip_uses_testcontainer(CreateAssignment, ListAssignmentInfos::Spec, ListAssignmentInfos::Chapters, GetAssignmentInfo, UpdateAssignmentRoles)(positive): assignment repo creates, lists, fetches, and updates roles in an isolated PostgreSQL container.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::assignment::{AssignmentEntry, AssignmentInfoListSpec, AssignmentRoleUpdate};
use crate::part::repo::oper::assignment::{CreateAssignment, GetAssignmentInfo, ListAssignmentInfos, UpdateAssignmentRoles};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::BaseError;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-domain-";

/// Verifies assignment roundtrip via testcontainers.
/// Verifies assignment roundtrip via testcontainers.
pub async fn assignment_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let assignee_form = test_shared::user_entry(PREFIX, "assignee");

    test_shared::create_user(&shared, &assignee_form).await;

    let translator_role = RoleMask::from(RoleField::TRANSLATOR);

    let reviewer_role = RoleMask::from(RoleField::REVIEWER);

    let assignment_entry = AssignmentEntry {
        id: format!("{}assignment", PREFIX),
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        user_id: assignee_form.id.clone(),
        roles: translator_role,
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreateAssignment {
                    entry: &assignment_entry,
                },
            )
            .await?;

            Ok::<(), BaseError>(())
        })
        .await
        .ok()
        .unwrap();

    let assignment_list_spec = AssignmentInfoListSpec::Chapter {
        chapter_id: chapter_fixture.chapter_entry.id.clone(),
        role: Some(RoleField::TRANSLATOR),
        incl_opt: vec![AssignmentInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let assignment_infos = repo
        .run(&ListAssignmentInfos::Spec {
            spec: &assignment_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(assignment_infos.len(), 1);

    assert_eq!(
        assignment_infos[0].user.as_ref().unwrap().id,
        assignee_form.id
    );

    let chapter_ids = vec![chapter_fixture.chapter_entry.id.clone()];

    let assignment_infos = repo
        .run(&ListAssignmentInfos::Chapters {
            chapter_ids: &chapter_ids,
            incls: &[],
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(assignment_infos.len(), 1);

    assert_eq!(assignment_infos[0].id, assignment_entry.id);

    let assignment_role_update = AssignmentRoleUpdate {
        id: assignment_entry.id.clone(),
        roles: reviewer_role,
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &UpdateAssignmentRoles {
                    update: &assignment_role_update,
                },
            )
            .await?;

            Ok::<(), BaseError>(())
        })
        .await
        .ok()
        .unwrap();

    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &assignment_entry.id,
            incls: &[AssignmentInclOpt::User],
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(assignment_info.roles, reviewer_role);

    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &assignment_entry.id,
            incls: &[AssignmentInclOpt::ChapterComicWorksetTeam],
        })
        .await
        .ok()
        .unwrap();

    let chapter_info = assignment_info.chapter.as_ref().unwrap();

    let comic_info = chapter_info.comic.as_ref().unwrap();

    assert_eq!(chapter_info.id, chapter_fixture.chapter_entry.id);

    assert_eq!(comic_info.id, chapter_fixture.comic_entry.id);

    assert_eq!(
        comic_info.workset.as_ref().unwrap().id,
        chapter_fixture.workset_entry.id
    );

    assert_eq!(
        comic_info.team.as_ref().unwrap().id,
        chapter_fixture.team_entry.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
