// team_roundtrip_uses_testcontainer(ListTeamInfos, UpdateTeam, GetTeamInfo)(positive): team repo persists, lists, and updates a team in an isolated PostgreSQL container.

use super::*;

use crate::model::team::{TeamInfoListKind, TeamInfoListSpec};
use crate::part::repo::oper::team::{GetTeamInfo, ListTeamInfos, UpdateTeam};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;

const PREFIX: &str = "rdb-test-team-domain-";

/// Verifies team roundtrip via testcontainers.
/// Verifies team roundtrip via testcontainers.
pub async fn team_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let team_info_list_spec = TeamInfoListSpec {
        kind: TeamInfoListKind::All,
        offset: 0,
        limit: 10,
    };

    let team_infos = repo
        .run(&ListTeamInfos {
            spec: &team_info_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert!(
        team_infos
            .iter()
            .any(|team_info| team_info.id == team_fixture.team_entry.id)
    );

    repo.run(&UpdateTeam::Info {
        id: &team_fixture.team_entry.id,
        name: "RDB Team Updated",
        description: "updated",
    })
    .await
    .ok()
    .unwrap();

    let team_info = repo
        .run(&GetTeamInfo::Id {
            id: &team_fixture.team_entry.id,
        })
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
