// workset_roundtrip_reads_test_database_url(WorksetRepo)(positive): workset repo persists, lists, and updates a workset in the local test database.

use super::*;

use poprako_orchestra::Run as _;

use poprako_util::page::Page;

use crate::part::repo::oper::workset::{
    GetWorksetInfo, ListWorksetInfos, UpdateWorkset,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::model::workset::WorksetInfoUpdate;

const PREFIX: &str = "rdb-test-workset-domain-";

#[tokio::test]
async fn workset_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let workset_fixture = test_shared::seed_workset(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());



    let workset_infos = repo.run(&ListWorksetInfos {
        team_id: &workset_fixture.team_entry.id,
        page: Some(Page {
            offset: 0,
            limit: 10,
        }),
    }).await.ok().unwrap();

    assert_eq!(workset_infos.len(), 1);

    let workset_info_update = WorksetInfoUpdate {
        id: workset_fixture.workset_entry.id.clone(),
        name: "RDB Workset Updated".into(),
        description: Some("updated".into()),
    };



    repo.run(&UpdateWorkset {
        update: &workset_info_update,
    }).await.ok().unwrap();



    let workset_info = repo.run(&GetWorksetInfo {
        id: &workset_fixture.workset_entry.id,
    }).await.ok().unwrap();

    assert_eq!(workset_info.name, "RDB Workset Updated");

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
