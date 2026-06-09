use futures_util::FutureExt as _;
use poprako_util::page::Page;
use tracing::instrument;

use crate::domain::complex::workset::WorksetComplex;
use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::workset::{WorksetAggr, WorksetForm, WorksetUpdate};
use crate::domain::query::Query;
use crate::domain::query::Transactional;
use crate::domain::query::team::TeamQueryTransactional;
use crate::domain::query::workset::WorksetQuery;
use crate::domain::query::workset::WorksetQueryTransactional;
use crate::usecase::data_object::workset::{
    WorksetBase, WorksetCreateParams, WorksetCreateReply, WorksetUpdateParams,
};
use crate::usecase::result::UseCaseResult;

#[instrument(err, skip(harn))]
pub async fn create<H>(harn: &H, params: WorksetCreateParams) -> UseCaseResult<WorksetCreateReply>
where
    H: Clone + Transactional + Query + ImageGet + Send + Sync,
{
    let id = WorksetAggr::generate_id();

    let (_workset, base) = Transactional::transaction_scoped(harn, move |query| {
        async move {
            // Atomically allocate the next index from the team-level sequence.
            let index =
                TeamQueryTransactional::increment_workset_next_index(query, &params.team_id)
                    .await?;

            let form = WorksetForm {
                id: id.clone(),
                team_id: params.team_id.clone(),
                index,
                name: params.name.clone(),
                description: params.description.clone(),
            };

            let created = WorksetQueryTransactional::create(query, &form).await?;

            Ok((created, BasePlaceholder { id: id.clone() }))
        }
        .boxed()
    })
    .await?;

    // Re-fetch to include team preload for the response.
    let aggr = WorksetQuery::get_by_id(harn, &base.id).await?;

    let base = WorksetBase::from_aggr(aggr, harn).await;

    Ok(WorksetCreateReply { id: base.id })
}

/// Internal holder to keep the workset id alive while assembling the base.
struct BasePlaceholder {
    id: String,
}

#[instrument(err, skip(harn))]
pub async fn get_by_id<H>(harn: &H, id: &str) -> UseCaseResult<WorksetBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let workset = WorksetQuery::get_by_id(harn, id).await?;

    let base = WorksetBase::from_aggr(workset, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn list<H>(harn: &H, team_id: &str, page: Page) -> UseCaseResult<Vec<WorksetBase>>
where
    H: Query + ImageGet + Send + Sync,
{
    let worksets = WorksetQuery::list(harn, team_id, page).await?;

    let mut bases = Vec::with_capacity(worksets.len());
    for workset in worksets {
        bases.push(WorksetBase::from_aggr(workset, harn).await);
    }

    Ok(bases)
}

#[instrument(err, skip(harn))]
pub async fn update<H>(
    harn: &H,
    workset_id: String,
    params: WorksetUpdateParams,
) -> UseCaseResult<()>
where
    H: Query + Send + Sync,
{
    let update = WorksetUpdate {
        id: workset_id,
        name: params.name,
        description: params.description,
    };

    WorksetQuery::update(harn, &update).await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn delete<H>(harn: &H, workset_id: String) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| {
        async move {
            WorksetComplex::delete_cascade(query, &workset_id).await?;
            Ok(())
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // create_workset_allocates_index_and_persists(create)(positive): create should allocate the next workset index and persist the workset.
    // create_two_worksets_allocates_sequential_indices(create)(positive): two creates should allocate sequential indices.
    // create_fails_for_nonexistent_team(create)(negative): create should fail when the team does not exist.
    // get_by_id_returns_workset_base(get_by_id)(positive): get_by_id should return a WorksetBase for an existing workset.
    // get_by_id_fails_for_nonexistent(get_by_id)(negative): get_by_id should fail with expected error for missing workset.
    // list_returns_worksets_for_team(list)(positive): list should return worksets for the given team.
    // list_empty_with_offset_past_end(list)(positive): list should return an empty vector when offset is past the last workset.
    // update_modifies_workset_fields(update)(positive): update should modify workset name and description.
    // update_fails_for_nonexistent(update)(negative): update should fail with expected error for missing workset.
    // delete_removes_workset(delete)(positive): delete should remove the workset.
    // delete_fails_for_nonexistent(delete)(negative): delete should fail with expected error for missing workset.

    use super::*;

    use time::OffsetDateTime;

    use crate::domain::model::aggr::team::TeamAggr;
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::usecase::data_object::workset::{WorksetCreateParams, WorksetUpdateParams};

    fn make_test_team(id: &str) -> TeamAggr {
        let now = OffsetDateTime::now_utc();
        TeamAggr {
            id: id.into(),
            name: "T".into(),
            description: "D".into(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            workset_next_index: 0,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn create_workset_allocates_index_and_persists() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let reply = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "WS1".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        let found = get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.name, "WS1");
        assert_eq!(found.index, 0);
    }

    #[tokio::test]
    async fn create_two_worksets_allocates_sequential_indices() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let r1 = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "A".into(),
                description: None,
            },
        )
        .await
        .unwrap();
        let r2 = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "B".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        let w1 = get_by_id(&harn, &r1.id).await.unwrap();
        let w2 = get_by_id(&harn, &r2.id).await.unwrap();
        assert_eq!(w1.index, 0);
        assert_eq!(w2.index, 1);
    }

    #[tokio::test]
    async fn get_by_id_fails_for_nonexistent() {
        let harn = TestHarness::default();
        let err = get_by_id(&harn, "no-such-workset").await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_returns_worksets_for_team() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));
        harn.seed_team(make_test_team("team-2"));

        let r1 = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "A".into(),
                description: None,
            },
        )
        .await
        .unwrap();
        let r2 = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "B".into(),
                description: None,
            },
        )
        .await
        .unwrap();
        create(
            &harn,
            WorksetCreateParams {
                team_id: "team-2".into(),
                name: "C".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        let list = super::list(
            &harn,
            "team-1",
            Page {
                offset: 0,
                limit: 10,
            },
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn update_modifies_workset_fields() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let reply = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "Old".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        update(
            &harn,
            reply.id.clone(),
            WorksetUpdateParams {
                name: "New".into(),
                description: Some("Updated desc".into()),
            },
        )
        .await
        .unwrap();

        let found = get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.name, "New");
        assert_eq!(found.description, Some("Updated desc".into()));
    }

    #[tokio::test]
    async fn update_fails_for_nonexistent() {
        let harn = TestHarness::default();
        let err = update(
            &harn,
            "no-such-workset".into(),
            WorksetUpdateParams {
                name: "X".into(),
                description: None,
            },
        )
        .await
        .err()
        .unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_workset() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let reply = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "ToDelete".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        delete(&harn, reply.id.clone()).await.unwrap();

        let err = get_by_id(&harn, &reply.id).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_fails_for_nonexistent() {
        let harn = TestHarness::default();
        let err = delete(&harn, "no-such-workset".into()).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn create_fails_for_nonexistent_team() {
        let harn = TestHarness::default();

        let err = create(
            &harn,
            WorksetCreateParams {
                team_id: "no-such-team".into(),
                name: "WS".into(),
                description: None,
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn get_by_id_returns_workset_base() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        let reply = create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "MyWS".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        let base = get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(base.name, "MyWS");
        assert_eq!(base.team_id, "team-1");
        assert_eq!(base.index, 0);
    }

    #[tokio::test]
    async fn list_empty_with_offset_past_end() {
        let harn = TestHarness::default();
        harn.seed_team(make_test_team("team-1"));

        create(
            &harn,
            WorksetCreateParams {
                team_id: "team-1".into(),
                name: "WS".into(),
                description: None,
            },
        )
        .await
        .unwrap();

        let list = super::list(
            &harn,
            "team-1",
            Page {
                offset: 10,
                limit: 5,
            },
        )
        .await
        .unwrap();
        assert!(list.is_empty());
    }
}
