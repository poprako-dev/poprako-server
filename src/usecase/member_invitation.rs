//! Member invitation use cases.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::MemberPermComplex;
use crate::complex::member_invitation::{MemberInvitationComplex, MemberInvitationPermComplex};
use crate::data::member_invitation::{
    CreateMemberInvitationData, CreateMemberInvitationVal, ListMemberInvitationInfosData,
    MemberInvitationInfoVal, UpdateMemberInvitationInfoData,
};
use crate::model::member_invitation::{MemberInvitationForm, MemberInvitationUpdate};
use crate::model::role::RoleMask;
use crate::model::user::UserToken;
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::proxy::{ProxyNonTransactional, ProxyTransactional};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::member_invitation::MemberInvitationStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateMemberInvitationData,
) -> RootResult<CreateMemberInvitationVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: MemberInvitationRepoTransactional<C>
        + MemberRepoTransactional<C>
        + UserRepoTransactional<C>
        + Send
        + Sync,
{
    let role_mask = RoleMask::try_from_bits(data.role_mask)?;

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let mut proxy = ProxyTransactional::new(&repo, context);
            MemberPermComplex::can_user_create_invitation(
                &mut proxy,
                &token.user_id,
                &data.team_id,
            )
            .await?;

            let invitee_user_info = repo
                .advance(context, &UserStep::find_info_by_qid(&data.invitee_qid))
                .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                let invitee_member_info = repo
                    .advance(
                        context,
                        &MemberStep::find_by_user_team_id(&invitee_user_info.id, &data.team_id),
                    )
                    .await?;

                if invitee_member_info.is_some() {
                    return Err(RootError::Expected {
                        variant: ExpectedVariant::Args,
                        message: trl("error-already-team-member"),
                    });
                }
            }

            let member_invitation_id = MemberInvitationComplex::gen_id();
            let member_invitation_code = MemberInvitationComplex::gen_code();

            let member_invitation_form = MemberInvitationForm {
                id: member_invitation_id,
                team_id: data.team_id,
                invitor_id: token.user_id,
                invitee_qid: data.invitee_qid,
                code: member_invitation_code,
                role_mask,
            };

            let member_invitation_info = repo
                .advance(
                    context,
                    &MemberInvitationStep::create(&member_invitation_form),
                )
                .await?;

            accept(CreateMemberInvitationVal {
                id: member_invitation_info.id,
                code: member_invitation_info.code,
            })
        })
        .await
        .map_err(map_drive_err)
}

pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListMemberInvitationInfosData,
) -> RootResult<Vec<MemberInvitationInfoVal>>
where
    R: MemberInvitationRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberInvitationRepoTransactional<C> + MemberRepoTransactional<C>,
{
    let mut proxy = ProxyNonTransactional::new(repo);

    MemberPermComplex::can_user_list_invitation_infos(
        &mut proxy,
        &token.user_id,
        &data.team_id,
    )
    .await?;

    let member_invitation_infos = repo
        .execute(&MemberInvitationStep::list_infos(
            &data.team_id,
            data.pending,
            data.offset,
            data.limit,
        ))
        .await?;

    Ok(member_invitation_infos
        .into_iter()
        .map(MemberInvitationInfoVal::from)
        .collect())
}

pub async fn update_info<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: UpdateMemberInvitationInfoData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberInvitationRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
{
    let role_mask = RoleMask::try_from_bits(data.role_mask)?;

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let member_invitation_info = repo
                .advance(context, &MemberInvitationStep::get_info_by_id(&data.id))
                .await?;

            let mut proxy = ProxyTransactional::new(&repo, context);

            MemberInvitationPermComplex::can_user_update_info(
                &mut proxy,
                &token.user_id,
                &member_invitation_info.team_id,
            )
            .await?;

            let member_invitation_update = MemberInvitationUpdate {
                id: data.id,
                role_mask,
            };

            repo.advance(
                context,
                &MemberInvitationStep::update_info(&member_invitation_update),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}

pub async fn delete<D, C, R>(drive: &D, repo: &R, token: UserToken, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberInvitationRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
{
    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let member_invitation_info = repo
                .advance(context, &MemberInvitationStep::get_info_by_id(&id))
                .await?;

            let mut proxy = ProxyTransactional::new(&repo, context);

            MemberInvitationPermComplex::can_user_delete(
                &mut proxy,
                &token.user_id,
                &member_invitation_info.team_id,
            )
            .await?;

            repo.advance(context, &MemberInvitationStep::delete(&id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}
