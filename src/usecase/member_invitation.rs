//! Member invitation use cases.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member_invitation::{MemberInvitationComplex, MemberInvitationPermComplex};
use crate::data::member_invitation::{
    CreateMemberInvitationData, CreateMemberInvitationVal, ListMemberInvitationInfosData,
    MemberInvitationInfoVal, UpdateMemberInvitationInfoData,
};
use crate::model::member_invitation::{MemberInvitationForm, MemberInvitationUpdate};
use crate::model::user::UserToken;
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::member_invitation::MemberInvitationStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Creates a pending invitation for a team（汉化组）.
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
    let role_mask = data.role_mask;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberInvitationPermComplex::can_user_create(
            &mut repo.as_proxy(),
            &token.user_id,
            &data.team_id,
        )
        .await?;
    }

    let (member_invitation_id, member_invitation_code) = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let invitee_user_info = repo
                .advance(context, &UserStep::find_info_by_qid(&data.invitee_qid))
                .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                let invitee_member_info = repo
                    .advance(
                        context,
                        &MemberStep::find_info_by_user_id_and_team_id(
                            &invitee_user_info.id,
                            &data.team_id,
                        ),
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

            accept((member_invitation_info.id, member_invitation_info.code))
        })
        .await
        .map_err(map_drive_err)?;

    accept(CreateMemberInvitationVal {
        id: member_invitation_id,
        code: member_invitation_code,
    })
}

/// Lists invitations for a team（汉化组）.
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
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberInvitationPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
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

/// Updates the role mask of an invitation.
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
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberInvitationPermComplex::can_user_update_info(
            &mut repo.as_proxy(),
            &token.user_id,
            &data.id,
        )
        .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let member_invitation_update = MemberInvitationUpdate {
                id: data.id,
                role_mask: data.role_mask,
            };

            repo.advance(
                context,
                &MemberInvitationStep::update_info(&member_invitation_update),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Deletes an invitation.
pub async fn delete<D, C, R>(drive: &D, repo: &R, token: UserToken, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: MemberInvitationRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberInvitationRepoTransactional<C> + MemberRepoTransactional<C> + Send + Sync,
{
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberInvitationPermComplex::can_user_delete(&mut repo.as_proxy(), &token.user_id, &id)
            .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(context, &MemberInvitationStep::delete(&id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
