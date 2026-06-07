use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
use crate::domain::model::value::role::{RoleFlag, RoleMask};
use crate::domain::query::Query;
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQuery;
use crate::domain::query::member::MemberQueryTransactional;
use crate::usecase::data_object::member::{
    MemberBase, MemberCreateParams, MemberCreateReply, MemberRoleUpdateParams,
};
use crate::usecase::result::UseCaseResult;

#[instrument(err, skip(harn))]
pub async fn create<H>(harn: &H, params: MemberCreateParams) -> UseCaseResult<MemberCreateReply>
where
    H: Clone + Transactional + Send + Sync,
{
    let id = MemberAggr::generate_id();
    let role_mask = RoleMask::from(params.role_mask);

    let form = MemberForm {
        id,
        user_id: params.user_id,
        user_nickname: String::new(),
        team_id: params.team_id,
        roles: role_mask,
    };

    let member = Transactional::transaction_scoped(harn, move |query| {
        async move { MemberQueryTransactional::create(query, &form).await }.boxed()
    })
    .await?;

    Ok(MemberCreateReply { id: member.id })
}

#[instrument(err, skip(harn))]
pub async fn get_by_id<H>(harn: &H, id: &str) -> UseCaseResult<MemberBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let member = MemberQuery::get_by_id(harn, id).await?;

    let base = MemberBase::from_aggr(member, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn get_by_user_and_team<H>(
    harn: &H,
    user_id: &str,
    team_id: &str,
) -> UseCaseResult<MemberBase>
where
    H: Query + ImageGet + Send + Sync,
{
    let member = MemberQuery::get_by_user_and_team_id(harn, user_id, team_id).await?;

    let base = MemberBase::from_aggr(member, harn).await;

    Ok(base)
}

#[instrument(err, skip(harn))]
pub async fn list<H>(
    harn: &H,
    team_id: &str,
    keyword: Option<&str>,
    role: Option<RoleFlag>,
    offset: i64,
    limit: i64,
) -> UseCaseResult<Vec<MemberBase>>
where
    H: Query + ImageGet + Send + Sync,
{
    let members = MemberQuery::list(harn, team_id, keyword, role, offset, limit).await?;

    let mut bases = Vec::with_capacity(members.len());
    for member in members {
        bases.push(MemberBase::from_aggr(member, harn).await);
    }

    Ok(bases)
}

#[instrument(err, skip(harn))]
pub async fn update_roles<H>(
    harn: &H,
    member_id: String,
    params: MemberRoleUpdateParams,
) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let update = MemberRoleUpdate {
        id: member_id,
        roles: RoleMask::from(params.roles),
    };

    Transactional::transaction_scoped(harn, move |query| {
        async move { MemberQueryTransactional::update_roles(query, &update).await }.boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn delete<H>(harn: &H, member_id: String) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    Transactional::transaction_scoped(harn, move |query| {
        let id = member_id.clone();
        async move { MemberQueryTransactional::delete(query, &id).await }.boxed()
    })
    .await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    // create_persists_member(create)(positive): create should persist a member.
    // get_by_id_fails_for_nonexistent(get_by_id)(negative): get_by_id should fail with expected error for missing member.
    // list_filters_by_team(list)(positive): list should filter members by team_id.
    // update_roles_modifies_all_roles(update_roles)(positive): update_roles should replace all role timestamps.
    // update_roles_fails_for_nonexistent(update_roles)(negative): update_roles should fail with expected error for missing member.
    // delete_removes_member(delete)(positive): delete should remove the member.
    // delete_fails_for_nonexistent(delete)(negative): delete should fail with expected error for missing member.

    use super::*;

    use time::OffsetDateTime;

    use crate::domain::model::aggr::member::MemberAggr;
    use crate::domain::model::aggr::team::TeamAggr;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::harness::tests::TestHarness;
    use crate::test_util::usecase_is_expected_argument;
    use crate::usecase::data_object::member::{MemberCreateParams, MemberRoleUpdateParams};

    fn make_test_member(id: &str, user_id: &str, team_id: &str, roles: RoleMask) -> MemberAggr {
        let now = OffsetDateTime::now_utc();
        let mut m = MemberAggr {
            id: id.into(),
            user_id: user_id.into(),
            user_nickname: "TestUser".into(),
            user: None,
            team_id: team_id.into(),
            team: None,
            assigned_raw_provider_at: None,
            assigned_translator_at: None,
            assigned_proofreader_at: None,
            assigned_typesetter_at: None,
            assigned_redrawer_at: None,
            assigned_reviewer_at: None,
            assigned_publisher_at: None,
            assigned_admin_at: None,
            assigned_assistant_at: None,
            user_last_active_at: now,
            created_at: now,
            updated_at: now,
        };

        let mask: u32 = roles.into();
        if mask & u32::from(RoleFlag::Admin) != 0 {
            m.assigned_admin_at = Some(now);
        }
        if mask & u32::from(RoleFlag::Translator) != 0 {
            m.assigned_translator_at = Some(now);
        }

        m
    }

    #[tokio::test]
    async fn create_persists_member() {
        let harn = TestHarness::default();
        harn.seed_team(TeamAggr {
            id: "team-1".into(),
            name: "T".into(),
            description: "D".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        });

        let reply = create(
            &harn,
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin) | u32::from(RoleFlag::Translator),
            },
        )
        .await
        .unwrap();

        let found = get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.user_id, "u-1");
        assert_eq!(found.team_id, "team-1");
        assert_eq!(
            found.roles,
            u32::from(RoleFlag::Admin) | u32::from(RoleFlag::Translator)
        );
    }

    #[tokio::test]
    async fn get_by_id_fails_for_nonexistent() {
        let harn = TestHarness::default();
        let err = get_by_id(&harn, "no-such-member").await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_filters_by_team() {
        let harn = TestHarness::default();
        harn.seed_member(make_test_member(
            "m-1",
            "u-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));
        harn.seed_member(make_test_member(
            "m-2",
            "u-2",
            "team-1",
            RoleFlag::Translator.into(),
        ));
        harn.seed_member(make_test_member(
            "m-3",
            "u-3",
            "team-2",
            RoleFlag::Admin.into(),
        ));

        let list = super::list(&harn, "team-1", None, None, 0, 10)
            .await
            .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn update_roles_modifies_all_roles() {
        let harn = TestHarness::default();
        harn.seed_team(TeamAggr {
            id: "team-1".into(),
            name: "T".into(),
            description: "D".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        });

        let reply = create(
            &harn,
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .unwrap();

        // Update to Translator only.
        update_roles(
            &harn,
            reply.id.clone(),
            MemberRoleUpdateParams {
                roles: u32::from(RoleFlag::Translator),
            },
        )
        .await
        .unwrap();

        let found = get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.roles, u32::from(RoleFlag::Translator));
    }

    #[tokio::test]
    async fn update_roles_fails_for_nonexistent() {
        let harn = TestHarness::default();
        let err = update_roles(
            &harn,
            "no-such-member".into(),
            MemberRoleUpdateParams { roles: 1 },
        )
        .await
        .err()
        .unwrap();
        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_removes_member() {
        let harn = TestHarness::default();
        harn.seed_team(TeamAggr {
            id: "team-1".into(),
            name: "T".into(),
            description: "D".into(),
            avatar_key: String::new(),
            avatar_uploaded: false,
            workset_next_index: 0,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        });

        let reply = create(
            &harn,
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
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
        let err = delete(&harn, "no-such-member".into()).await.err().unwrap();
        assert!(usecase_is_expected_argument(&err));
    }
}
