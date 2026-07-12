// announcement_roundtrip_reads_test_database_url(AnnouncementStep)(positive): announcement repo creates and lists included users in the local test database.

use super::*;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::announcement_model;
use crate::part::repo::step::announcement::AnnouncementStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::announcement::AnnouncementInclOpt;

const PREFIX: &str = "rdb-test-announcement-domain-";

#[tokio::test]
async fn announcement_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let announcement_form = announcement_model::Form {
        id: format!("{}announcement", PREFIX),
        team_id: team_fixture.team_form.id.clone(),
        user_id: team_fixture.user_form.id.clone(),
        title: "RDB Announcement".into(),
        content: "announcement".into(),
    };

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &AnnouncementStep::create(&announcement_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let announcement_list_spec = announcement_model::ListSpec {
        team_id: team_fixture.team_form.id.clone(),
        incl_opt: vec![AnnouncementInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let announcement_infos = Execute::execute(
        &repo,
        &AnnouncementStep::list_infos(&announcement_list_spec),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(announcement_infos.len(), 1);

    assert_eq!(
        announcement_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_form.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
