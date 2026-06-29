//! Member use cases — create, list, role update, and deletion.

// NOTE: get_by_code API 去除（TODO：删除 note）

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member::{
    CreateMemberData, CreateMemberVal, ListMemberInfosData, MemberInfoVal, UpdateMemberRoleData,
};
use crate::model::member::{MemberForm, MemberListSpec, MemberRoleUpdate};
use crate::model::user::UserToken;
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Creates one member under a team.
///
/// The caller must be a team admin. The target user and team are locked in
/// the transaction before inserting the membership.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateMemberData,
) -> RootResult<CreateMemberVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberRepo<C> + TeamRepo<C> + UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C>
        + TeamRepoTransactional<C>
        + UserRepoTransactional<C>
        + Send
        + Sync,
{
    let role_mask = data.role_mask;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberPermComplex::can_user_create(&mut repo.as_proxy(), &token.user_id, &data.team_id)
            .await?;
    }

    let member_id = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let target_user_info = repo
                .advance(context, &UserStep::get_info_excluded(&data.user_id))
                .await?;

            repo.advance(context, &TeamStep::get_info_excluded(&data.team_id))
                .await?;

            let existing_member_info = repo
                .advance(
                    context,
                    &MemberStep::find_info_by_user_id_and_team_id(&data.user_id, &data.team_id),
                )
                .await?;

            if existing_member_info.is_some() {
                return Err(RootError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-already-team-member"),
                });
            }

            let member_form = MemberForm {
                id: MemberComplex::gen_id(),
                user_id: data.user_id,
                user_nickname: target_user_info.nickname,
                team_id: data.team_id,
                roles: role_mask,
            };

            let member_info = repo
                .advance(context, &MemberStep::create(&member_form))
                .await?;

            accept(member_info.id)
        })
        .await
        .map_err(map_drive_err)?;

    accept(CreateMemberVal { id: member_id })
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListMemberInfosData,
) -> RootResult<Vec<MemberInfoVal>>
where
    R: MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C>,
{
    let member_list_spec: MemberListSpec = data.try_into()?;

    if let MemberListSpec::Team { team_id, .. } = &member_list_spec {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberPermComplex::can_user_list_infos(&mut repo.as_proxy(), &token.user_id, team_id)
            .await?;
    }

    let member_infos = repo
        .execute(&MemberStep::list_infos(&member_list_spec))
        .await?;

    Ok(member_infos.into_iter().map(MemberInfoVal::from).collect())
}

/// Updates one member's role mask.
///
/// The caller must be a team admin of the target member's team.
pub async fn update_role<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: UpdateMemberRoleData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C> + Send + Sync,
{
    let target_member_info = repo.execute(&MemberStep::get_info_by_id(&data.id)).await?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberPermComplex::can_user_update_info(
            &mut repo.as_proxy(),
            &token.user_id,
            &target_member_info.team_id,
        )
        .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let member_role_update = MemberRoleUpdate {
                id: data.id,
                roles: data.roles,
            };

            repo.advance(context, &MemberStep::update_role(&member_role_update))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Deletes one member.
///
/// The caller must be a team admin of the target member's team.
pub async fn delete<D, C, R>(drive: &D, repo: &R, token: UserToken, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C> + Send + Sync,
{
    let target_member_info = repo.execute(&MemberStep::get_info_by_id(&id)).await?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberPermComplex::can_user_delete(
            &mut repo.as_proxy(),
            &token.user_id,
            &target_member_info.team_id,
        )
        .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(context, &MemberStep::delete(&id)).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
