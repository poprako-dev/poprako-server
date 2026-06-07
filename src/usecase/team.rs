use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::external::image_pool::{ImageGet, ImagePut};
use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamUpdate};
use crate::domain::query::Query;
use crate::domain::query::Transactional;
use crate::domain::query::team::TeamQuery;
use crate::domain::query::team::TeamQueryTransactional;
use crate::usecase::data_object::team::{
    ReserveTeamAvatarParams, ReserveTeamAvatarReply, TeamBase, TeamCreateParams, TeamUpdateParams,
};
use crate::usecase::result::UseCaseResult;

#[instrument(err, skip(harn))]
pub async fn create<H>(harn: &H, params: TeamCreateParams) -> UseCaseResult<TeamBase>
where
    H: Clone + Transactional + ImageGet + Send + Sync,
{
    let id = TeamAggr::generate_id();

    let form = TeamForm {
        id,
        name: params.name,
        description: params.description,
    };

    let team = Transactional::transaction_scoped(harn, move |query| {
        async move { TeamQueryTransactional::create(query, &form).await }.boxed()
    })
    .await?;

    let base = TeamBase::from_aggr(team, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn get_info<H>(harn: &H, id: &str) -> UseCaseResult<TeamBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let team = TeamQuery::get_by_id(harn, id).await?;

    let base = TeamBase::from_aggr(team, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn list<H>(harn: &H, offset: i64, limit: i64) -> UseCaseResult<Vec<TeamBase>>
where
    H: Query + ImageGet + Send + Sync,
{
    let teams = TeamQuery::list(harn, offset, limit).await?;

    let mut bases = Vec::with_capacity(teams.len());
    for team in teams {
        bases.push(TeamBase::from_aggr(team, harn).await);
    }

    Ok(bases)
}

#[instrument(err, skip(harn))]
pub async fn update<H>(harn: &H, team_id: String, params: TeamUpdateParams) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let input = TeamUpdate {
        id: team_id,
        name: params.name,
        description: params.description,
    };

    Transactional::transaction_scoped(harn, move |query| {
        async move { TeamQueryTransactional::update(query, &input).await }.boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn reserve_avatar<H>(
    harn: &H,
    team_id: String,
    params: ReserveTeamAvatarParams,
) -> UseCaseResult<ReserveTeamAvatarReply>
where
    H: Query + ImagePut + Send + Sync,
{
    // Fetch team just to call generate_avatar_key.
    let team = TeamQuery::get_by_id(harn, &team_id).await?;
    let avatar_key = team.generate_avatar_key(&params.file_extension);

    TeamQuery::prefill_avatar_key(harn, &team_id, &avatar_key).await?;

    let put_url = ImagePut::put_signed(harn, &avatar_key).await?.to_string();

    Ok(ReserveTeamAvatarReply { put_url })
}

#[instrument(err, skip(harn))]
pub async fn mark_avatar_uploaded<H>(harn: &H, team_id: String) -> UseCaseResult<()>
where
    H: Query + Send + Sync,
{
    TeamQuery::mark_avatar_uploaded(harn, &team_id).await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn delete<H>(harn: &H, team_id: String) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| {
        let id = team_id.clone();
        async move { TeamQueryTransactional::delete(query, &id).await }.boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // create_team_persists_and_returns_team_base(create)(positive): create should persist a team and return a TeamBase.
    // get_info_fails_for_nonexistent_team(get_info)(negative): get_info should fail with expected error for missing team.
    // list_returns_all_teams(list)(positive): list should return all seeded teams.
    // update_modifies_team_fields(update)(positive): update should modify team name and description.
    // update_fails_for_nonexistent_team(update)(negative): update should fail with expected error for missing team.
    // delete_removes_team(delete)(positive): delete should remove the team.
    // delete_fails_for_nonexistent_team(delete)(negative): delete should fail with expected error for missing team.
    // reserve_avatar_generates_key_and_put_url(reserve_avatar)(positive): reserve_avatar should generate an avatar key and a signed PUT URL.
    // mark_avatar_uploaded_sets_flag(mark_avatar_uploaded)(positive): mark_avatar_uploaded should set the avatar_uploaded flag.

    use super::*;

    use time::OffsetDateTime;

    use crate::domain::model::aggr::team::TeamAggr;
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::usecase::data_object::team::{TeamCreateParams, TeamUpdateParams};

    fn make_test_team(id: &str) -> TeamAggr {
        let now = time::OffsetDateTime::now_utc();
        TeamAggr {
            id: id.into(),
            name: "Test Team".into(),
            description: "A test team".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_team_persists_and_returns_team_base() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "My Team".into(),
                description: "Desc".into(),
            },
        )
        .await
        .unwrap();

        assert_eq!(base.name, "My Team");
        assert_eq!(base.description, "Desc");

        let found = get_info(&harn, &base.id).await.unwrap();
        assert_eq!(found.name, "My Team");
    }

    #[tokio::test]
    async fn get_info_fails_for_nonexistent_team() {
        let harn = TestHarness::default();
        let err = get_info(&harn, "no-such-team").await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_returns_all_teams() {
        let harn = TestHarness::default();

        // Seed two teams directly into the mock.
        harn.seed_team(make_test_team("team-a"));
        harn.seed_team(make_test_team("team-b"));

        let teams = list(&harn, 0, 10).await.unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn update_modifies_team_fields() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "Old".into(),
                description: "Old desc".into(),
            },
        )
        .await
        .unwrap();

        update(
            &harn,
            base.id.clone(),
            TeamUpdateParams {
                name: "New".into(),
                description: "New desc".into(),
            },
        )
        .await
        .unwrap();

        let found = get_info(&harn, &base.id).await.unwrap();
        assert_eq!(found.name, "New");
        assert_eq!(found.description, "New desc");
    }

    #[tokio::test]
    async fn update_fails_for_nonexistent_team() {
        let harn = TestHarness::default();
        let err = update(
            &harn,
            "no-such-team".into(),
            TeamUpdateParams {
                name: "X".into(),
                description: "Y".into(),
            },
        )
        .await
        .err()
        .unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_team() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "ToDelete".into(),
                description: "X".into(),
            },
        )
        .await
        .unwrap();

        delete(&harn, base.id.clone()).await.unwrap();

        let err = get_info(&harn, &base.id).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_fails_for_nonexistent_team() {
        let harn = TestHarness::default();
        let err = delete(&harn, "no-such-team".into()).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn reserve_avatar_generates_key_and_put_url() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "AvatarTeam".into(),
                description: "X".into(),
            },
        )
        .await
        .unwrap();

        let reply = reserve_avatar(
            &harn,
            base.id.clone(),
            ReserveTeamAvatarParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        assert!(reply.put_url.contains("put"));
        assert!(reply.put_url.contains("team_avatar"));
        assert!(reply.put_url.contains("png"));
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_sets_flag() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "MarkAvatar".into(),
                description: "X".into(),
            },
        )
        .await
        .unwrap();

        // First reserve to set avatar_key.
        reserve_avatar(
            &harn,
            base.id.clone(),
            ReserveTeamAvatarParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        mark_avatar_uploaded(&harn, base.id.clone()).await.unwrap();

        let found = get_info(&harn, &base.id).await.unwrap();
        assert!(found.avatar_url.is_some());
    }
}
