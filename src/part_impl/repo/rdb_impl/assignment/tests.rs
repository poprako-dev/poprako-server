// assignment_roundtrip_reads_test_database_url(AssignmentStep)(positive): assignment repo creates, lists, fetches, and updates roles in the local test database.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::assignment::{
    AssignmentForm, AssignmentListSpec, AssignmentRoleUpdate,
};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::assignment::AssignmentInclOpt;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-domain-";

#[tokio::test]
async fn assignment_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let assignee_form = test_shared::user_form(PREFIX, "assignee");

    test_shared::create_user(&shared, &assignee_form).await;

    let translator_role = RoleMask::from(RoleField::TRANSLATOR);

    let reviewer_role = RoleMask::from(RoleField::REVIEWER);

    let assignment_form = AssignmentForm {
        id: format!("{}assignment", PREFIX),
        chapter_id: chapter_fixture.chapter_form.id.clone(),
        user_id: assignee_form.id.clone(),
        roles: translator_role,
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &AssignmentStep::create(&assignment_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let assignment_list_spec = AssignmentListSpec::Chapter {
        chapter_id: chapter_fixture.chapter_form.id.clone(),
        role: Some(RoleField::TRANSLATOR),
        incl_opt: vec![AssignmentInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let assignment_infos = Execute::execute(
        &repo,
        &AssignmentStep::list_infos(&assignment_list_spec),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(assignment_infos.len(), 1);
    assert_eq!(
        assignment_infos[0].user.as_ref().unwrap().id,
        assignee_form.id
    );

    let assignment_role_update = AssignmentRoleUpdate {
        id: assignment_form.id.clone(),
        roles: reviewer_role,
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &AssignmentStep::put_roles(&assignment_role_update),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let assignment_info = Execute::execute(
        &repo,
        &AssignmentStep::get_info_by_id(
            &assignment_form.id,
            &[AssignmentInclOpt::User],
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(assignment_info.roles, reviewer_role);

    let assignment_info = Execute::execute(
        &repo,
        &AssignmentStep::get_info_by_id(
            &assignment_form.id,
            &[AssignmentInclOpt::ChapterComicWorksetTeam],
        ),
    )
    .await
    .ok()
    .unwrap();

    let chapter_info = assignment_info.chapter.as_ref().unwrap();

    let comic_info = chapter_info.comic.as_ref().unwrap();

    assert_eq!(chapter_info.id, chapter_fixture.chapter_form.id);
    assert_eq!(comic_info.id, chapter_fixture.comic_form.id);
    assert_eq!(
        comic_info.workset.as_ref().unwrap().id,
        chapter_fixture.workset_form.id
    );
    assert_eq!(
        comic_info.team.as_ref().unwrap().id,
        chapter_fixture.team_form.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
