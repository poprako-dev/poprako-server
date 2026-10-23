#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// assignment_roundtrip_uses_testcontainer(CreateAssignment, ListAssignmentInfos::Spec, ListAssignmentInfos::Chapters, GetAssignmentInfo, UpdateAssignmentRoles)(positive): assignment repo creates, lists, fetches, and updates roles in an isolated PostgreSQL container.

use super::*;

use poprako_orchestra::Nucl as _;

use poprako_rdb_core::RdbCore;

use crate::config::AppConfig;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::model::write::assignment::AssignmentRoleRepl;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::assignment::{
    CreateAssignment, GetAssignmentInfo, ListAssignmentInfos,
    UpdateAssignmentRoles,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::ChapterFixture;
use crate::result::BaseError;
use crate::shape::write::assignment::AssignmentEntryShape;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-domain-";

// Verify the included assignment resolves every ancestor through the owning team.
async fn verify_assignment_ancestor_inclusions(
    repo: &HybRepo,
    chapter_fixture: &ChapterFixture,
    assignment_entry: &AssignmentEntryShape<'_>,
) {
    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &assignment_entry.id,
            incls: &[AssignmentInclOpt::ChapterComicWorksetTeam],
        })
        .await
        .unwrap();

    let chapter_info = assignment_info.chapter.as_ref().unwrap();

    let comic_info = chapter_info.comic.as_ref().unwrap();

    assert_eq!(chapter_info.id, chapter_fixture.chapter_info.id);

    assert_eq!(comic_info.id, chapter_fixture.comic_info.id);

    assert_eq!(
        comic_info.workset.as_ref().unwrap().id,
        chapter_fixture.workset_entry.id
    );

    assert_eq!(
        comic_info.team.as_ref().unwrap().id,
        chapter_fixture.team_entry.id
    );
}

/// Verifies assignment roundtrip via testcontainers.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn assignment_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let config = AppConfig::default();

    let repo = HybRepo::new(shared.clone(), config.cache, config.repo);

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let assignee_form = test_shared::form::user_entry(PREFIX, "assignee");

    test_shared::create_user(&shared, &assignee_form).await;

    let reviewer_role = RoleMask::from(RoleField::REVIEWER);

    let assignment_entry = AssignmentEntryShape {
        id: format!("{}assignment", PREFIX),
        chapter_id: &chapter_fixture.chapter_info.id,
        user_id: &assignee_form.id,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    };

    nucl.coord(async |context| {
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
    .unwrap();

    let assignment_list_spec = AssignmentListSpec::Chapter {
        chapter_id: chapter_fixture.chapter_info.id.clone(),
        role: Some(RoleField::TRANSLATOR),
        incl_opt: vec![AssignmentInclOpt::User],
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let assignment_infos = repo
        .run(&ListAssignmentInfos::Spec {
            spec: &assignment_list_spec,
        })
        .await
        .unwrap();

    assert_eq!(assignment_infos.len(), 1);

    assert_eq!(
        assignment_infos
            .as_slice()
            .first()
            .and_then(|info| info.user.as_ref())
            .unwrap()
            .id,
        assignee_form.id
    );

    let chapter_ids = vec![chapter_fixture.chapter_info.id.as_str()];

    let assignment_infos = repo
        .run(&ListAssignmentInfos::Chapters {
            chapter_ids: &chapter_ids,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(assignment_infos.len(), 1);

    assert_eq!(
        assignment_infos.as_slice().first().unwrap().id,
        assignment_entry.id
    );

    let assignment_role_update = AssignmentRoleRepl {
        id: assignment_entry.id.clone(),
        roles: reviewer_role,
    };

    nucl.coord(async |context| {
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
    .unwrap();

    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &assignment_entry.id,
            incls: &[AssignmentInclOpt::User],
        })
        .await
        .unwrap();

    assert_eq!(assignment_info.roles, reviewer_role);

    verify_assignment_ancestor_inclusions(
        &repo,
        &chapter_fixture,
        &assignment_entry,
    )
    .await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}
