// team_roundtrip_reads_test_database_url(TeamStep)(positive): team repo persists, lists, and updates a team in the local test database.

use poprako_util::page::Page;

use crate::part::repo::step::team::TeamStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-team-domain-";

#[tokio::test]
async fn team_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let page = Page {
        offset: 0,
        limit: 10,
    };

    let team_infos = Execute::execute(&repo, &TeamStep::list_infos(None, page))
        .await
        .ok()
        .unwrap();

    assert!(
        team_infos
            .iter()
            .any(|team_info| team_info.id == team_fixture.team_form.id)
    );

    Execute::execute(
        &repo,
        &TeamStep::update_info(
            &team_fixture.team_form.id,
            "RDB Team Updated",
            "updated",
        ),
    )
    .await
    .ok()
    .unwrap();

    let team_info = Execute::execute(
        &repo,
        &TeamStep::get_info_by_id(&team_fixture.team_form.id),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(team_info.name, "RDB Team Updated");

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
