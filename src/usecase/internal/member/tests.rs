use super::*;

use poprako_orchestra::Nucl as _;
use time::OffsetDateTime;

use crate::model::read::proj::member::MemberInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::test_util::fixture::workset;
use crate::value::role::{RoleField, RoleMask};

fn member(user_id: &str, team_id: &str) -> MemberInfo {
    MemberInfo {
        id: "member-1".into(),
        user_id: user_id.into(),
        user_nickname: "member".into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: team_id.into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

#[tokio::test]
async fn workset_loader_returns_same_info_in_run_and_step_modes() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(member("user-1", "team-1"));

    let run_info = MemberLoader::load_info_from_workset(
        &mock,
        LoadMode::Run,
        "user-1",
        "workset-1",
    )
    .await
    .ok()
    .unwrap();

    let step_info = mock
        .coord(async |context| {
            MemberLoader::load_info_from_workset(
                &mock,
                LoadMode::Step { context },
                "user-1",
                "workset-1",
            )
            .await
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(run_info.id, step_info.id);

    assert_eq!(run_info.user_id, step_info.user_id);

    assert_eq!(run_info.team_id, step_info.team_id);
}

#[tokio::test]
async fn workset_loader_rejects_missing_membership_before_pure_rules() {
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    let error = MemberLoader::load_info_from_workset(
        &mock,
        LoadMode::Run,
        "user-1",
        "workset-1",
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(error, ExpectedVariant::Perm);
}
