use futures_util::FutureExt as _;
use poprako_util::page::Page;
use time::Duration;
use tracing::instrument;

use crate::domain::complex;
use crate::domain::external::image_pool::{ImageGet, ImagePut};
use crate::domain::local_message::message::{ImageLocalMessage, ImageResourceKind};
use crate::domain::model::aggr::team::{TeamAggr, TeamForm, TeamInfoUpdate};
use crate::domain::query::local_message::LocalMessageQueryTransactional;
use crate::domain::query::team::{TeamQuery, TeamQueryTransactional};
use crate::domain::query::{Query, Transactional};
use crate::usecase::data_object::team::{
    TeamAvatarMarkUploadedParams, TeamAvatarReserveParams, TeamAvatarReserveReply, TeamBase,
    TeamCreateParams, TeamInfoUpdateParams,
};
use crate::usecase::result::UseCaseResult;

#[instrument(err, skip(harn))]
pub async fn create<H>(harn: &H, params: TeamCreateParams) -> UseCaseResult<TeamBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let id = TeamAggr::generate_id();

    let form = TeamForm {
        id,
        name: params.name,
        description: params.description,
    };

    let team = TeamQuery::create(harn, &form).await?;

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
pub async fn list<H>(harn: &H, page: Page) -> UseCaseResult<Vec<TeamBase>>
where
    H: Query + ImageGet + Send + Sync,
{
    let teams = TeamQuery::list(harn, page).await?;

    let mut bases = Vec::with_capacity(teams.len());
    for team in teams {
        bases.push(TeamBase::from_aggr(team, harn).await);
    }

    Ok(bases)
}

#[instrument(err, skip(harn))]
pub async fn update_info<H>(
    harn: &H,
    team_id: String,
    params: TeamInfoUpdateParams,
) -> UseCaseResult<()>
where
    H: Query + Send + Sync,
{
    let update = TeamInfoUpdate {
        id: team_id,
        name: params.name,
        description: params.description,
    };

    TeamQuery::update_info(harn, &update).await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn reserve_avatar<H>(
    harn: &H,
    team_id: String,
    params: TeamAvatarReserveParams,
) -> UseCaseResult<TeamAvatarReserveReply>
where
    H: Clone + Transactional + ImagePut + Send + Sync,
{
    let reservation = Transactional::transaction_scoped(harn, move |query| {
        async move {
            let reservation =
                TeamQueryTransactional::reserve_avatar(query, &team_id, &params.file_extension)
                    .await?;

            if let Some(previous_object_key) = reservation.previous_object_key.clone() {
                let message =
                    ImageLocalMessage::delete(previous_object_key).into_form(Duration::seconds(0));
                LocalMessageQueryTransactional::append(query, &message).await?;
            }

            let message = ImageLocalMessage::check_uploaded(
                ImageResourceKind::TeamAvatar,
                team_id,
                reservation.object_key.clone(),
                reservation.image_version,
            )
            .into_form(Duration::minutes(15));
            LocalMessageQueryTransactional::append(query, &message).await?;

            Ok(reservation)
        }
        .boxed()
    })
    .await?;

    let put_url = ImagePut::put_signed(harn, &reservation.object_key)
        .await?
        .to_string();

    Ok(TeamAvatarReserveReply {
        put_url,
        image_version: reservation.image_version,
    })
}

#[instrument(err, skip(harn))]
pub async fn mark_avatar_uploaded<H>(
    harn: &H,
    team_id: String,
    params: TeamAvatarMarkUploadedParams,
) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| {
        async move {
            TeamQueryTransactional::mark_avatar_uploaded(query, &team_id, params.image_version)
                .await?;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn delete<H>(harn: &H, team_id: String) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| {
        async move {
            complex::team::delete_cascade(query, &team_id).await?;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // create_team_persists_and_returns_team_base(create)(positive): create should persist a team and return a TeamBase.
    // get_info_fails_for_nonexistent_team(get_info)(negative): get_info should fail with expected error for missing team.
    // list_returns_all_teams(list)(positive): list should return all seeded teams.
    // update_info_modifies_team_fields(update_info)(positive): update_info should modify team name and description.
    // update_info_fails_for_nonexistent_team(update_info)(negative): update_info should fail with expected error for missing team.
    // delete_removes_team(delete)(positive): delete should remove the team.
    // delete_fails_for_nonexistent_team(delete)(negative): delete should fail with expected error for missing team.
    // delete_queues_avatar_cleanup_message(delete)(positive): deleting a team with a reserved avatar should queue a local message to delete the old avatar object.
    // delete_cascade_deletes_worksets(delete)(positive): deleting a team should cascade-delete all worksets belonging to the team.
    // reserve_avatar_generates_key_and_put_url(reserve_avatar)(positive): reserve_avatar should generate an avatar key and a signed PUT URL.
    // mark_avatar_uploaded_sets_flag(mark_avatar_uploaded)(positive): mark_avatar_uploaded should set the avatar_uploaded flag.
    // create_duplicate_name_returns_conflict(create)(negative): create should fail with conflict when team name already exists.
    // get_info_returns_team_base(get_info)(positive): get_info should return a TeamBase for an existing team.
    // list_empty_with_offset_past_end(list)(positive): list should return an empty vector when offset is past the last team.
    // reserve_avatar_fails_for_nonexistent_team(reserve_avatar)(negative): reserve_avatar should fail for missing team.
    // mark_avatar_uploaded_fails_for_nonexistent_team(mark_avatar_uploaded)(negative): mark_avatar_uploaded should fail for missing team.
    // mark_avatar_uploaded_fails_for_stale_version(mark_avatar_uploaded)(negative): mark_avatar_uploaded should fail when image_version does not match.

    use super::*;

    use crate::domain::local_message::message::{ImageLocalMessage, ImageResourceKind};
    use crate::domain::model::aggr::team::TeamAggr;
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::test_util::usecase_is_expected_conflict;
    use crate::usecase::data_object::team::{
        TeamAvatarMarkUploadedParams, TeamCreateParams, TeamInfoUpdateParams,
    };
    use crate::usecase::data_object::workset::WorksetCreateParams;
    use crate::usecase::workset;

    fn make_test_team(id: &str) -> TeamAggr {
        let now = time::OffsetDateTime::now_utc();
        TeamAggr {
            id: id.into(),
            name: "Test Team".into(),
            description: "A test team".into(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
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

        let teams = list(
            &harn,
            Page {
                offset: 0,
                limit: 10,
            },
        )
        .await
        .unwrap();
        assert_eq!(teams.len(), 2);
    }

    #[tokio::test]
    async fn update_info_modifies_team_fields() {
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

        update_info(
            &harn,
            base.id.clone(),
            TeamInfoUpdateParams {
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
    async fn update_info_fails_for_nonexistent_team() {
        let harn = TestHarness::default();
        let err = update_info(
            &harn,
            "no-such-team".into(),
            TeamInfoUpdateParams {
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
    async fn delete_queues_avatar_cleanup_message() {
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

        // Reserve an avatar so the team has an avatar_key.
        reserve_avatar(
            &harn,
            base.id.clone(),
            TeamAvatarReserveParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        let before_messages = harn.snapshot().local_messages.len();

        delete(&harn, base.id.clone()).await.unwrap();

        // Team is gone.
        let err = get_info(&harn, &base.id).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));

        // A delete local message was queued for the avatar.
        let snapshot = harn.snapshot();
        assert_eq!(snapshot.local_messages.len(), before_messages + 1);

        let new_msg = snapshot.local_messages.last().unwrap();
        let message: ImageLocalMessage = serde_json::from_value(new_msg.payload.clone()).unwrap();
        match message {
            ImageLocalMessage::Delete { object_key, .. } => {
                assert!(object_key.contains("team_avatar"));
            }
            other => panic!(
                "expected Delete message, got {:?}",
                std::mem::discriminant(&other)
            ),
        }
    }

    #[tokio::test]
    async fn delete_cascade_deletes_worksets() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "CascadeTeam".into(),
                description: "X".into(),
            },
        )
        .await
        .unwrap();

        // Create two worksets under this team via the usecase.
        let r1 = workset::create(
            &harn,
            WorksetCreateParams {
                team_id: base.id.clone(),
                name: "WS1".into(),
                description: None,
            },
        )
        .await
        .unwrap();
        let r2 = workset::create(
            &harn,
            WorksetCreateParams {
                team_id: base.id.clone(),
                name: "WS2".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        delete(&harn, base.id.clone()).await.unwrap();

        // Team is gone.
        let err = get_info(&harn, &base.id).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));

        // Both worksets should be cascade-deleted.
        let err = workset::get_by_id(&harn, &r1.id).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
        let err = workset::get_by_id(&harn, &r2.id).await.err().unwrap();
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
            TeamAvatarReserveParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        assert!(reply.put_url.contains("put"));
        assert!(reply.put_url.contains("team_avatar"));
        assert!(reply.put_url.contains("png"));
        assert_eq!(reply.image_version, 1);

        let snapshot = harn.snapshot();
        assert_eq!(snapshot.local_messages.len(), 1);
        let message: ImageLocalMessage =
            serde_json::from_value(snapshot.local_messages[0].payload.clone()).unwrap();
        match message {
            ImageLocalMessage::CheckUploaded {
                resource_kind,
                resource_id,
                object_key,
                image_version,
            } => {
                assert_eq!(resource_kind, ImageResourceKind::TeamAvatar);
                assert_eq!(resource_id, base.id);
                assert_eq!(object_key, format!("team_avatar/{}-1.png", resource_id));
                assert_eq!(image_version, 1);
            }
            ImageLocalMessage::Delete { .. } => panic!("expected check-upload message"),
        }
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
        let reply = reserve_avatar(
            &harn,
            base.id.clone(),
            TeamAvatarReserveParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        mark_avatar_uploaded(
            &harn,
            base.id.clone(),
            TeamAvatarMarkUploadedParams {
                image_version: reply.image_version,
            },
        )
        .await
        .unwrap();

        let found = get_info(&harn, &base.id).await.unwrap();
        assert!(found.avatar_url.is_some());
    }

    #[tokio::test]
    async fn create_duplicate_name_returns_conflict() {
        let harn = TestHarness::default();

        create(
            &harn,
            TeamCreateParams {
                name: "Dupe".into(),
                description: "First".into(),
            },
        )
        .await
        .unwrap();

        let err = create(
            &harn,
            TeamCreateParams {
                name: "Dupe".into(),
                description: "Second".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn get_info_returns_team_base() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let base = get_info(&harn, "team-1").await.unwrap();
        assert_eq!(base.id, "team-1");
        assert_eq!(base.name, "Test Team");
    }

    #[tokio::test]
    async fn list_empty_with_offset_past_end() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let teams = list(
            &harn,
            Page {
                offset: 10,
                limit: 5,
            },
        )
        .await
        .unwrap();
        assert!(teams.is_empty());
    }

    #[tokio::test]
    async fn reserve_avatar_fails_for_nonexistent_team() {
        let harn = TestHarness::default();

        let err = reserve_avatar(
            &harn,
            "no-such-team".into(),
            TeamAvatarReserveParams {
                file_extension: "png".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_fails_for_nonexistent_team() {
        let harn = TestHarness::default();

        let err = mark_avatar_uploaded(
            &harn,
            "no-such-team".into(),
            TeamAvatarMarkUploadedParams { image_version: 1 },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn mark_avatar_uploaded_fails_for_stale_version() {
        let harn = TestHarness::default();

        let base = create(
            &harn,
            TeamCreateParams {
                name: "StaleVer".into(),
                description: "X".into(),
            },
        )
        .await
        .unwrap();

        reserve_avatar(
            &harn,
            base.id.clone(),
            TeamAvatarReserveParams {
                file_extension: "png".into(),
            },
        )
        .await
        .unwrap();

        let err = mark_avatar_uploaded(
            &harn,
            base.id.clone(),
            TeamAvatarMarkUploadedParams { image_version: 999 },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }
}
