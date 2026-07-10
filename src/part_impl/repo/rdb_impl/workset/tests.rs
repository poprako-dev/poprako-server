// workset_roundtrip_reads_test_database_url(WorksetStep)(positive): workset repo persists, lists, and updates a workset in the local test database.

use super::*;

use poprako_util::page::Page;

use crate::model::workset::WorksetInfoUpdate;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-workset-domain-";

#[tokio::test]
async fn workset_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let workset_fixture = test_shared::seed_workset(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let page = Page {
        offset: 0,
        limit: 10,
    };

    let workset_infos = Execute::execute(
        &repo,
        &WorksetStep::list_infos_by_team_id(
            &workset_fixture.team_form.id,
            page,
        ),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(workset_infos.len(), 1);

    let workset_info_update = WorksetInfoUpdate {
        id: workset_fixture.workset_form.id.clone(),
        name: "RDB Workset Updated".into(),
        description: Some("updated".into()),
    };

    Execute::execute(&repo, &WorksetStep::update_info(&workset_info_update))
        .await
        .ok()
        .unwrap();

    let workset_info = Execute::execute(
        &repo,
        &WorksetStep::get_info_by_id(&workset_fixture.workset_form.id),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(workset_info.name, "RDB Workset Updated");

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
