//! Member use cases: create, join, list, role update, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member::{
    CreateMemberData, CreateMemberVal, JoinTeamData, ListMemberInfosData, MemberInfoVal,
    UpdateMemberRoleData,
};
use crate::model::member::{MemberForm, MemberListSpec, MemberRoleUpdate};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::member_invitation::MemberInvitationStep;
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
    let roles = data.roles;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberPermComplex::can_user_create(&mut repo.as_proxy(), &token.user_id, &data.team_id).await?;

    let member_id = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

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
                    variant: ExpectedVariant::ArgsInvalid,
                    message: trl("error-already-team-member"),
                });
            }

            let member_form = MemberForm {
                id: MemberComplex::gen_id(),
                user_id: data.user_id,
                user_nickname: target_user_info.nickname,
                team_id: data.team_id,
                roles: roles,
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

/// Joins the current user to a team with a pending invitation code.
pub async fn join_team<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: JoinTeamData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberRepo<C> + MemberInvitationRepo<C> + UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C>
        + MemberInvitationRepoTransactional<C>
        + UserRepoTransactional<C>
        + Send
        + Sync,
{
    let current_user_id = token.user_id;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let current_user_info = repo
                .advance(context, &UserStep::get_info_excluded(&current_user_id))
                .await?;

            let member_invitation_info = repo
                .advance(
                    context,
                    &MemberInvitationStep::get_info_by_code_excluded(&data.code),
                )
                .await?;

            if member_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_error());
            }

            let existing_member_info = repo
                .advance(
                    context,
                    &MemberStep::find_info_by_user_id_and_team_id(
                        &current_user_id,
                        &member_invitation_info.team_id,
                    ),
                )
                .await?;

            if existing_member_info.is_some() {
                return Err(already_team_member_error());
            }

            let member_form = MemberForm {
                id: MemberComplex::gen_id(),
                user_id: current_user_id,
                user_nickname: current_user_info.nickname,
                team_id: member_invitation_info.team_id.clone(),
                roles: member_invitation_info.roles,
            };

            repo.advance(context, &MemberStep::create(&member_form))
                .await?;

            repo.advance(
                context,
                &MemberInvitationStep::mark_pending_as_used(&member_invitation_info.id),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListMemberInfosData,
) -> RootResult<Vec<MemberInfoVal>>
where
    R: MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C>,
    I: ImagePool,
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

    let mut member_info_vals = Vec::with_capacity(member_infos.len());

    for member_info in member_infos {
        member_info_vals.push(MemberInfoVal::from_model(image_pool, member_info).await?);
    }

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
    let target_member_info = repo.execute(&MemberStep::get_info_by_id(&data.id)).await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberPermComplex::can_user_update_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &target_member_info.team_id,
    )
    .await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

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

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberPermComplex::can_user_delete(
        &mut repo.as_proxy(),
        &token.user_id,
        &target_member_info.team_id,
    )
    .await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            repo.advance(context, &MemberStep::delete(&id)).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

fn invalid_invitation_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl("error-no-pending-invitation"),
    }
}

fn already_team_member_error() -> RootError {
    RootError::Expected {
        variant: ExpectedVariant::ArgsInvalid,
        message: trl("error-already-team-member"),
    }
}
