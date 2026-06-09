use futures_util::FutureExt as _;
use poprako_util::i18n::trl;
use tracing::instrument;

use crate::domain::external::image_pool::ImageGet;
use crate::domain::model::aggr::member::{MemberAggr, MemberForm, MemberRoleUpdate};
use crate::domain::model::aggr::user::UserToken;
use crate::domain::model::value::role::{RoleFlag, RoleMask};
use crate::domain::query::Query;
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQuery;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::query::team::TeamQueryTransactional;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::{DomainError, DomainResult};
use crate::usecase::data_object::member::{
    MemberCreateParams, MemberCreateReply, MemberInfo, MemberJoinParams, MemberListParams,
    MemberRoleUpdateParams,
};
use crate::usecase::result::UseCaseResult;

fn validate_role_mask(mask: u32) -> DomainResult<RoleMask> {
    RoleMask::try_from(mask)
        .map_err(|_| DomainError::expected_argument(trl("error-member-not-found")))
}

async fn members_to_infos<H>(members: Vec<MemberAggr>, harn: &H) -> Vec<MemberInfo>
where
    H: ImageGet,
{
    let mut infos = Vec::with_capacity(members.len());
    for member in members {
        infos.push(MemberInfo::from_aggr(member, harn).await);
    }

    infos
}

#[instrument(err, skip(harn))]
pub async fn create<H>(
    harn: &H,
    user_token: &UserToken,
    params: MemberCreateParams,
) -> UseCaseResult<MemberCreateReply>
where
    H: Clone + Transactional + Send + Sync,
{
    let role_mask = validate_role_mask(params.role_mask)?;
    let id = MemberAggr::generate_id();
    let target_user_id = params.user_id;
    let target_team_id = params.team_id;

    let current_user_id = user_token.user_id.clone();

    let member = Transactional::transaction_scoped(harn, move |query| {
        async move {
            // Verify current user is sadmin.
            let current_user =
                UserQueryTransactional::get_by_id_excluded(query, &current_user_id).await?;
            if !current_user.is_sadmin {
                return Err(DomainError::expected_forbidden(trl(
                    "error-sadmin-required",
                )));
            }

            // Read target user to get real nickname.
            let target_user =
                UserQueryTransactional::get_by_id_excluded(query, &target_user_id).await?;

            // Verify team exists.
            let _team = TeamQueryTransactional::get_by_id_excluded(query, &target_team_id).await?;

            let form = MemberForm {
                id,
                user_id: target_user_id,
                user_nickname: target_user.nickname,
                team_id: target_team_id,
                roles: role_mask,
            };

            MemberQueryTransactional::create(query, &form).await
        }
        .boxed()
    })
    .await?;

    Ok(MemberCreateReply { id: member.id })
}

#[instrument(err, skip(harn, params))]
pub async fn list_infos<H>(
    harn: &H,
    user_token: &UserToken,
    params: &MemberListParams,
) -> UseCaseResult<Vec<MemberInfo>>
where
    H: Query + ImageGet + Send + Sync,
{
    if let Some(team_id) = &params.team_id {
        let is_member =
            MemberQuery::exist_by_user_and_team_id(harn, &user_token.user_id, team_id).await?;
        if !is_member {
            return Err(DomainError::expected_forbidden(trl("error-team-member-required")).into());
        }
    }

    let members = MemberQuery::list(
        harn,
        params.team_id.as_deref(),
        params.user_id.as_deref(),
        params.keyword.as_deref(),
        params.role,
        params.page,
        &params.includes,
    )
    .await?;

    Ok(members_to_infos(members, harn).await)
}

#[instrument(err, skip(harn))]
pub async fn update_roles<H>(
    harn: &H,
    user_token: &UserToken,
    member_id: String,
    params: MemberRoleUpdateParams,
) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let role_mask = validate_role_mask(params.roles)?;

    let current_user_id = user_token.user_id.clone();

    Transactional::transaction_scoped(harn, move |query| {
        async move {
            // Lock the target member.
            let target_member =
                MemberQueryTransactional::get_by_id_excluded(query, &member_id).await?;

            // Verify current user is an admin of the target member's team.
            let current_member = MemberQueryTransactional::get_by_user_and_team_id_excluded(
                query,
                &current_user_id,
                &target_member.team_id,
            )
            .await?;

            if !current_member.has_any_role(&[RoleFlag::Admin]) {
                return Err(DomainError::expected_forbidden(trl(
                    "error-team-admin-required",
                )));
            }

            let update = MemberRoleUpdate {
                id: member_id,
                roles: role_mask,
            };

            MemberQueryTransactional::update_roles(query, &update).await
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn delete<H>(harn: &H, user_token: &UserToken, member_id: String) -> UseCaseResult<()>
where
    H: Clone + Transactional + Send + Sync,
{
    let current_user_id = user_token.user_id.clone();

    Transactional::transaction_scoped(harn, move |query| {
        async move {
            // Lock the target member.
            let target_member =
                MemberQueryTransactional::get_by_id_excluded(query, &member_id).await?;

            // Verify current user is an admin of the target member's team.
            let current_member = MemberQueryTransactional::get_by_user_and_team_id_excluded(
                query,
                &current_user_id,
                &target_member.team_id,
            )
            .await?;

            if !current_member.has_any_role(&[RoleFlag::Admin]) {
                return Err(DomainError::expected_forbidden(trl(
                    "error-team-admin-required",
                )));
            }

            MemberQueryTransactional::delete(query, &member_id).await
        }
        .boxed()
    })
    .await?;

    Ok(())
}

#[instrument(err, skip(harn))]
pub async fn join<H>(
    harn: &H,
    user_token: &UserToken,
    params: MemberJoinParams,
) -> UseCaseResult<MemberCreateReply>
where
    H: Clone + Transactional + Send + Sync,
{
    let invitation_code = params.invitation_code;

    let current_user_id = user_token.user_id.clone();

    let member = Transactional::transaction_scoped(harn, move |query| {
        async move {
            // Lock the pending invitation by code.
            let invitation =
                MemberInvitationQueryTransactional::get_by_code_ex(query, &invitation_code).await?;

            // Read current user to verify qid matches.
            let current_user =
                UserQueryTransactional::get_by_id_excluded(query, &current_user_id).await?;

            // Verify invitation belongs to the current user by qid.
            if current_user.qid != invitation.invitee_qid {
                return Err(DomainError::expected_argument(trl(
                    "error-no-pending-invitation",
                )));
            }

            // Verify current user is not already a member of the target team.
            let already = MemberQueryTransactional::get_by_user_and_team_id_excluded(
                query,
                &current_user_id,
                &invitation.team_id,
            )
            .await;
            if already.is_ok() {
                return Err(DomainError::expected_conflict(trl(
                    "error-already-team-member",
                )));
            }

            // Create the member record.
            let form = MemberForm {
                id: MemberAggr::generate_id(),
                user_id: current_user_id,
                user_nickname: current_user.nickname,
                team_id: invitation.team_id,
                roles: invitation.roles,
            };

            let new_member = MemberQueryTransactional::create(query, &form).await?;

            // Mark invitation as used.
            MemberInvitationQueryTransactional::mark_pending_as_used(query, &invitation.id).await?;

            Ok(new_member)
        }
        .boxed()
    })
    .await?;

    Ok(MemberCreateReply { id: member.id })
}

#[cfg(test)]
mod tests {
    // create_sadmin_succeeds(create)(positive): sadmin should be able to create a member.
    // create_nonsadmin_returns_forbidden(create)(negative): non-sadmin should get forbidden.
    // create_fills_target_user_nickname(create)(positive): the target user's nickname should be written to the member.
    // create_target_user_not_found_fails(create)(negative): nonexistent target user should fail.
    // create_target_team_not_found_fails(create)(negative): nonexistent target team should fail.
    // create_duplicate_user_team_returns_conflict(create)(negative): duplicate user+team pair should return conflict.
    // list_team_member_succeeds(list_infos)(positive): team member should be able to list.
    // list_nonmember_returns_forbidden(list_infos)(negative): non-member should get forbidden.
    // list_includes_user_fills_user(list_infos)(positive): includes=user should fill the user field.
    // list_includes_team_fills_team(list_infos)(positive): includes=team should fill the team field.
    // list_infos_user_filter_returns_user_members(list_infos)(positive): user filter should return only that user's memberships.
    // update_roles_admin_succeeds(update_roles)(positive): team admin should be able to update roles.
    // update_roles_nonadmin_returns_forbidden(update_roles)(negative): non-admin should get forbidden.
    // update_roles_target_member_not_found_fails(update_roles)(negative): nonexistent target member should fail.
    // update_roles_zero_role_mask_fails(update_roles)(negative): zero role mask should fail.
    // delete_admin_succeeds(delete)(positive): team admin should be able to delete a member.
    // delete_nonadmin_returns_forbidden(delete)(negative): non-admin should get forbidden.
    // delete_target_member_not_found_fails(delete)(negative): nonexistent target member should fail.
    // join_by_code_succeeds(join)(positive): should join by invitation code.
    // join_code_not_found_fails(join)(negative): nonexistent code should fail.
    // join_wrong_qid_fails(join)(negative): mismatched qid should fail.
    // join_already_member_returns_conflict(join)(negative): already a member should return conflict.

    use super::*;

    use poprako_util::page::Page;
    use time::OffsetDateTime;

    use crate::domain::model::aggr::member::MemberAggr;
    use crate::domain::model::aggr::member_invitation::MemberInvitationAggr;
    use crate::domain::model::aggr::team::TeamAggr;
    use crate::domain::model::aggr::user::{UserAggr, UserCredential, UserToken};
    use crate::domain::model::value::member_inclusion::MemberInclusion;
    use crate::domain::model::value::role::{RoleFlag, RoleMask};
    use crate::domain::query::member::MemberQuery;
    use crate::harness::tests::TestHarness;
    use crate::test_util::is_expected_argument;
    use crate::test_util::usecase_is_expected_argument;
    use crate::test_util::usecase_is_expected_conflict;
    use crate::test_util::usecase_is_expected_forbidden;
    use crate::usecase::data_object::member::{
        MemberCreateParams, MemberJoinParams, MemberListParams, MemberRoleUpdateParams,
    };

    fn make_user(id: &str, qid: &str, is_sadmin: bool) -> UserAggr {
        let now = OffsetDateTime::now_utc();
        UserAggr {
            id: id.into(),
            qid: qid.into(),
            nickname: "nick".into(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            is_sadmin,
            last_active_at: now,
            created_at: now,
            updated_at: now,
        }
    }

    fn make_credential(user_id: &str) -> UserCredential {
        UserCredential {
            user_id: user_id.into(),
            password_hash: bcrypt::hash("pw", bcrypt::DEFAULT_COST).unwrap(),
        }
    }

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

    fn make_team(id: &str) -> TeamAggr {
        TeamAggr {
            id: id.into(),
            name: "T".into(),
            description: "D".into(),
            avatar_key: None,
            avatar_uploaded: false,
            avatar_version: 0,
            workset_next_index: 0,
            created_at: OffsetDateTime::now_utc(),
            updated_at: OffsetDateTime::now_utc(),
        }
    }

    fn sadmin_token() -> UserToken {
        UserToken {
            user_id: "sadmin".into(),
        }
    }

    fn user_token(user_id: &str) -> UserToken {
        UserToken {
            user_id: user_id.into(),
        }
    }

    #[tokio::test]
    async fn create_sadmin_succeeds() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("sadmin", "sadmin-qid", true),
            make_credential("sadmin"),
        );
        harn.seed_user(make_user("u-1", "u-1-qid", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));

        let reply = create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .unwrap();

        let found = MemberQuery::get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.user_id, "u-1");
        assert_eq!(found.team_id, "team-1");
    }

    #[tokio::test]
    async fn create_nonsadmin_returns_forbidden() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("regular", "reg-qid", false),
            make_credential("regular"),
        );
        harn.seed_user(make_user("u-1", "u-1-qid", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));

        let err = create(
            &harn,
            &user_token("regular"),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_forbidden(&err));
    }

    #[tokio::test]
    async fn create_fills_target_user_nickname() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("sadmin", "sadmin-qid", true),
            make_credential("sadmin"),
        );
        harn.seed_user(
            {
                let mut u = make_user("u-1", "u-1-qid", false);
                u.nickname = "RealNick".into();
                u
            },
            make_credential("u-1"),
        );
        harn.seed_team(make_team("team-1"));

        let reply = create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .unwrap();

        let found = MemberQuery::get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.user_nickname, "RealNick");
    }

    #[tokio::test]
    async fn create_target_user_not_found_fails() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("sadmin", "sadmin-qid", true),
            make_credential("sadmin"),
        );
        harn.seed_team(make_team("team-1"));

        let err = create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "no-such-user".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn create_target_team_not_found_fails() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("sadmin", "sadmin-qid", true),
            make_credential("sadmin"),
        );
        harn.seed_user(make_user("u-1", "u-1-qid", false), make_credential("u-1"));

        let err = create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "no-such-team".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn create_duplicate_user_team_returns_conflict() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("sadmin", "sadmin-qid", true),
            make_credential("sadmin"),
        );
        harn.seed_user(make_user("u-1", "u-1-qid", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));

        create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .unwrap();

        let err = create(
            &harn,
            &sadmin_token(),
            MemberCreateParams {
                user_id: "u-1".into(),
                team_id: "team-1".into(),
                role_mask: u32::from(RoleFlag::Translator),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_conflict(&err));
    }

    #[tokio::test]
    async fn list_team_member_succeeds() {
        let harn = TestHarness::default();
        harn.seed_user(make_user("u-1", "qid-1", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));
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

        let list = super::list_infos(
            &harn,
            &user_token("u-1"),
            &MemberListParams {
                team_id: Some("team-1".into()),
                user_id: None,
                keyword: None,
                role: None,
                page: Page {
                    offset: 0,
                    limit: 10,
                },
                includes: MemberInclusion::default(),
            },
        )
        .await
        .unwrap();
        assert_eq!(list.len(), 2);
    }

    #[tokio::test]
    async fn list_nonmember_returns_forbidden() {
        let harn = TestHarness::default();
        harn.seed_user(make_user("u-1", "qid-1", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));

        let err = super::list_infos(
            &harn,
            &user_token("u-1"),
            &MemberListParams {
                team_id: Some("team-1".into()),
                user_id: None,
                keyword: None,
                role: None,
                page: Page {
                    offset: 0,
                    limit: 10,
                },
                includes: MemberInclusion::default(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_forbidden(&err));
    }

    #[tokio::test]
    async fn update_roles_admin_succeeds() {
        let harn = TestHarness::default();
        let admin_user_id = "admin-1";
        let target_user_id = "target-1";
        let team_id = "team-1";

        harn.seed_user(
            make_user(admin_user_id, "admin-qid", false),
            make_credential(admin_user_id),
        );
        harn.seed_user(
            make_user(target_user_id, "target-qid", false),
            make_credential(target_user_id),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-admin",
            admin_user_id,
            team_id,
            RoleFlag::Admin.into(),
        ));
        harn.seed_member(make_test_member(
            "m-target",
            target_user_id,
            team_id,
            RoleFlag::Translator.into(),
        ));

        super::update_roles(
            &harn,
            &user_token(admin_user_id),
            "m-target".into(),
            MemberRoleUpdateParams {
                roles: u32::from(RoleFlag::Proofreader),
            },
        )
        .await
        .unwrap();

        let found = MemberQuery::get_by_id(&harn, "m-target").await.unwrap();
        assert!(!found.has_any_role(&[RoleFlag::Translator]));
        assert!(found.has_any_role(&[RoleFlag::Proofreader]));
    }

    #[tokio::test]
    async fn update_roles_nonadmin_returns_forbidden() {
        let harn = TestHarness::default();
        let team_id = "team-1";

        harn.seed_user(
            make_user("regular", "reg-qid", false),
            make_credential("regular"),
        );
        harn.seed_user(
            make_user("target", "target-qid", false),
            make_credential("target"),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-regular",
            "regular",
            team_id,
            RoleFlag::Translator.into(),
        ));
        harn.seed_member(make_test_member(
            "m-target",
            "target",
            team_id,
            RoleFlag::Translator.into(),
        ));

        let err = super::update_roles(
            &harn,
            &user_token("regular"),
            "m-target".into(),
            MemberRoleUpdateParams {
                roles: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_forbidden(&err));
    }

    #[tokio::test]
    async fn update_roles_target_member_not_found_fails() {
        let harn = TestHarness::default();
        let team_id = "team-1";

        harn.seed_user(
            make_user("admin", "admin-qid", false),
            make_credential("admin"),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-admin",
            "admin",
            team_id,
            RoleFlag::Admin.into(),
        ));

        let err = super::update_roles(
            &harn,
            &user_token("admin"),
            "no-such-member".into(),
            MemberRoleUpdateParams {
                roles: u32::from(RoleFlag::Admin),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn update_roles_zero_role_mask_fails() {
        let harn = TestHarness::default();

        let err = super::update_roles(
            &harn,
            &user_token("anyone"),
            "m-1".into(),
            MemberRoleUpdateParams { roles: 0 },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_admin_succeeds() {
        let harn = TestHarness::default();
        let team_id = "team-1";

        harn.seed_user(
            make_user("admin", "admin-qid", false),
            make_credential("admin"),
        );
        harn.seed_user(
            make_user("target", "target-qid", false),
            make_credential("target"),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-admin",
            "admin",
            team_id,
            RoleFlag::Admin.into(),
        ));
        harn.seed_member(make_test_member(
            "m-target",
            "target",
            team_id,
            RoleFlag::Translator.into(),
        ));

        super::delete(&harn, &user_token("admin"), "m-target".into())
            .await
            .unwrap();

        let err = MemberQuery::get_by_id(&harn, "m-target")
            .await
            .err()
            .unwrap();
        assert!(is_expected_argument(&err));
    }

    #[tokio::test]
    async fn delete_nonadmin_returns_forbidden() {
        let harn = TestHarness::default();
        let team_id = "team-1";

        harn.seed_user(
            make_user("regular", "reg-qid", false),
            make_credential("regular"),
        );
        harn.seed_user(
            make_user("target", "target-qid", false),
            make_credential("target"),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-regular",
            "regular",
            team_id,
            RoleFlag::Translator.into(),
        ));
        harn.seed_member(make_test_member(
            "m-target",
            "target",
            team_id,
            RoleFlag::Translator.into(),
        ));

        let err = super::delete(&harn, &user_token("regular"), "m-target".into())
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_forbidden(&err));
    }

    #[tokio::test]
    async fn delete_target_member_not_found_fails() {
        let harn = TestHarness::default();
        let team_id = "team-1";

        harn.seed_user(
            make_user("admin", "admin-qid", false),
            make_credential("admin"),
        );
        harn.seed_team(make_team(team_id));
        harn.seed_member(make_test_member(
            "m-admin",
            "admin",
            team_id,
            RoleFlag::Admin.into(),
        ));

        let err = super::delete(&harn, &user_token("admin"), "no-such-member".into())
            .await
            .err()
            .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn list_infos_user_filter_returns_user_members() {
        let harn = TestHarness::default();
        harn.seed_user(make_user("u-1", "qid-1", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));
        harn.seed_team(make_team("team-2"));
        harn.seed_member(make_test_member(
            "m-1",
            "u-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));
        harn.seed_member(make_test_member(
            "m-2",
            "u-1",
            "team-2",
            RoleFlag::Translator.into(),
        ));
        harn.seed_member(make_test_member(
            "m-3",
            "u-2",
            "team-1",
            RoleFlag::Admin.into(),
        ));

        let list = super::list_infos(
            &harn,
            &user_token("u-1"),
            &MemberListParams {
                team_id: None,
                user_id: Some("u-1".into()),
                keyword: None,
                role: None,
                page: Page {
                    offset: 0,
                    limit: 10,
                },
                includes: MemberInclusion::default(),
            },
        )
        .await
        .unwrap();

        assert_eq!(list.len(), 2);
        assert!(list.iter().all(|m| m.user_id == "u-1"));
    }

    #[tokio::test]
    async fn join_by_code_succeeds() {
        let harn = TestHarness::default();
        harn.seed_user(
            {
                let mut u = make_user("u-1", "invitee-qid", false);
                u.nickname = "MyNick".into();
                u
            },
            make_credential("u-1"),
        );
        harn.seed_team(make_team("team-1"));

        let invitation = MemberInvitationAggr {
            id: MemberInvitationAggr::generate_id(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: "invitee-qid".into(),
            code: "CODE123".into(),
            pending: true,
            roles: RoleMask::from(RoleFlag::Translator),
            created_at: OffsetDateTime::now_utc(),
        };
        harn.seed_invitation(invitation);

        let reply = super::join(
            &harn,
            &user_token("u-1"),
            MemberJoinParams {
                invitation_code: "CODE123".into(),
            },
        )
        .await
        .unwrap();

        let found = MemberQuery::get_by_id(&harn, &reply.id).await.unwrap();
        assert_eq!(found.user_id, "u-1");
        assert_eq!(found.team_id, "team-1");
        assert_eq!(found.user_nickname, "MyNick");
    }

    #[tokio::test]
    async fn join_code_not_found_fails() {
        let harn = TestHarness::default();
        harn.seed_user(make_user("u-1", "qid-1", false), make_credential("u-1"));

        let err = super::join(
            &harn,
            &user_token("u-1"),
            MemberJoinParams {
                invitation_code: "NO-SUCH-CODE".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn join_wrong_qid_fails() {
        let harn = TestHarness::default();
        harn.seed_user(make_user("u-1", "wrong-qid", false), make_credential("u-1"));
        harn.seed_team(make_team("team-1"));

        let invitation = MemberInvitationAggr {
            id: MemberInvitationAggr::generate_id(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: "right-qid".into(),
            code: "CODE123".into(),
            pending: true,
            roles: RoleMask::from(RoleFlag::Translator),
            created_at: OffsetDateTime::now_utc(),
        };
        harn.seed_invitation(invitation);

        let err = super::join(
            &harn,
            &user_token("u-1"),
            MemberJoinParams {
                invitation_code: "CODE123".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_argument(&err));
    }

    #[tokio::test]
    async fn join_already_member_returns_conflict() {
        let harn = TestHarness::default();
        harn.seed_user(
            make_user("u-1", "invitee-qid", false),
            make_credential("u-1"),
        );
        harn.seed_team(make_team("team-1"));
        harn.seed_member(make_test_member(
            "m-1",
            "u-1",
            "team-1",
            RoleFlag::Admin.into(),
        ));

        let invitation = MemberInvitationAggr {
            id: MemberInvitationAggr::generate_id(),
            invitor_id: "invitor-1".into(),
            invitor: None,
            team_id: "team-1".into(),
            invitee_qid: "invitee-qid".into(),
            code: "CODE123".into(),
            pending: true,
            roles: RoleMask::from(RoleFlag::Translator),
            created_at: OffsetDateTime::now_utc(),
        };
        harn.seed_invitation(invitation);

        let err = super::join(
            &harn,
            &user_token("u-1"),
            MemberJoinParams {
                invitation_code: "CODE123".into(),
            },
        )
        .await
        .err()
        .unwrap();

        assert!(usecase_is_expected_conflict(&err));
    }
}
