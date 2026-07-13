// team_roundtrip_reads_test_database_url(ListTeamInfos, UpdateTeam, GetTeamInfo)(positive): team repo persists, lists, and updates a team in the local test database.

use super::*;

use poprako_orchestra::Run as _;

use crate::part::repo::oper::team::{GetTeamInfo, ListTeamInfos, UpdateTeam};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-team-domain-";

#[tokio::test]
async fn team_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let team_infos = repo
        .run(&ListTeamInfos {
            user_id: None,
            offset: 0,
            limit: 10,
        })
        .await
        .ok()
        .unwrap();

    assert!(
        team_infos
            .iter()
            .any(|team_info| team_info.id == team_fixture.team_entry.id)
    );

    let update_team = UpdateTeam::Info {
        id: &team_fixture.team_entry.id,
        name: "RDB Team Updated",
        description: "updated",
    };

    repo.run(&update_team).await.ok().unwrap();

    let get_team_info = GetTeamInfo::Id {
        id: &team_fixture.team_entry.id,
    };

    let team_info = repo.run(&get_team_info).await.ok().unwrap();

    assert_eq!(team_info.name, "RDB Team Updated");

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
