// assignment_invitation_roundtrip_reads_test_database_url(AssignmentInvitationStep)(positive): assignment invitation repo creates, lists, and marks invitations used in the local test database.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;

use crate::model::assignment_invitation::AssignmentInvitationForm;
use crate::part::repo::step::assignment_invitation::AssignmentInvitationStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive_rdb::RdbDrive;
use crate::part_impl::repo_rdb::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::role::{RoleField, RoleMask};

const PREFIX: &str = "rdb-test-assignment-invitation-domain-";

#[tokio::test]
async fn assignment_invitation_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let assignment_invitation_form = AssignmentInvitationForm {
        id: format!("{}assignment-invitation", PREFIX),
        chapter_id: chapter_fixture.chapter_form.id.clone(),
        inviter_id: chapter_fixture.creator_form.id.clone(),
        invitee_qid: format!("{}invitee", PREFIX),
        code: format!("{}code", PREFIX),
        roles: RoleMask::from(RoleField::REVIEWER),
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &AssignmentInvitationStep::create(&assignment_invitation_form),
            )
            .await?;

            Advance::advance(
                &transactional_repo,
                context,
                &AssignmentInvitationStep::mark_pending_as_used(
                    &assignment_invitation_form.id,
                ),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let page = Page {
        offset: 0,
        limit: 10,
    };

    let assignment_invitation_infos = Execute::execute(
        &repo,
        &AssignmentInvitationStep::list_infos(
            &chapter_fixture.chapter_form.id,
            Some(false),
            page,
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(assignment_invitation_infos.len(), 1);
    assert!(!assignment_invitation_infos[0].pending);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
