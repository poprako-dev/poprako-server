//! Member use cases — create, list, role update, and deletion.

// NOTE: get_by_code API 去除（TODO：删除 note）

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member::{
    CreateMemberData, CreateMemberVal, ListMemberInfosData, ListMineMemberInfosData, MemberInfoVal,
    UpdateMemberRoleData,
};
use crate::model::member::{MemberForm, MemberListSpec, MemberRoleUpdate};
use crate::model::role::RoleMask;
use crate::model::user::UserToken;
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::proxy::{ProxyNonTransactional, ProxyTransactional};
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
    let role_mask = RoleMask::try_from_bits(data.role_mask)?;

    let member_id = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);

            MemberPermComplex::can_user_create(&mut proxy, &token.user_id, &data.team_id).await?;

            let target_user_info = repo
                .advance(context, &UserStep::get_info_excluded(&data.user_id))
                .await?;

            repo.advance(context, &TeamStep::get_info_excluded(&data.team_id))
                .await?;

            let existing_member_info = repo
                .advance(
                    context,
                    &MemberStep::find_by_user_team_id(&data.user_id, &data.team_id),
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
                role_mask,
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
    let role_mask = data.role_mask.map(RoleMask::try_from_bits).transpose()?;

    let member_list_spec = MemberListSpec {
        team_id: data.team_id,
        user_nickname_keyword: data.user_nickname_keyword,
        role_mask,
        offset: data.offset,
        limit: data.limit,
    };

    let mut proxy = ProxyNonTransactional::new(repo);

    MemberPermComplex::can_user_list_infos(&mut proxy, &token.user_id, &member_list_spec.team_id)
        .await?;

    let member_infos = repo
        .execute(&MemberStep::list_infos(&member_list_spec))
        .await?;

    Ok(member_infos.into_iter().map(MemberInfoVal::from).collect())
}

/// Lists memberships of the current user.
///
/// Uses the transactional list step because the repository exposes the
/// user-scoped member list through the locked path.
pub async fn list_mine_infos<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: ListMineMemberInfosData,
) -> RootResult<Vec<MemberInfoVal>>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C> + Send + Sync,
{
    let member_info_vals = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let member_infos = repo
                .advance(
                    context,
                    &MemberStep::list_by_user_id_excluded(&token.user_id),
                )
                .await?;

            let member_info_vals = member_infos
                .into_iter()
                .skip(data.offset as usize)
                .take(data.limit as usize)
                .map(MemberInfoVal::from)
                .collect();

            accept(member_info_vals)
        })
        .await
        .map_err(map_drive_err)?;

    accept(member_info_vals)
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
    let role_mask = RoleMask::try_from_bits(data.role_mask)?;

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let target_member_info = repo
                .advance(context, &MemberStep::get_info_excluded(&data.id))
                .await?;

            let mut proxy = ProxyTransactional::new(&repo, context);

            MemberPermComplex::can_user_update_info(
                &mut proxy,
                &token.user_id,
                &target_member_info.team_id,
            )
            .await?;

            let member_role_update = MemberRoleUpdate {
                id: data.id,
                role_mask,
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
    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let target_member_info = repo
                .advance(context, &MemberStep::get_info_excluded(&id))
                .await?;

            let mut proxy = ProxyTransactional::new(&repo, context);

            MemberPermComplex::can_user_delete(
                &mut proxy,
                &token.user_id,
                &target_member_info.team_id,
            )
            .await?;

            repo.advance(context, &MemberStep::delete(&id)).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
