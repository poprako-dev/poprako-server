//! Member use cases: create, join, list, role update, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member_data;
use crate::model::member_model;
use crate::model::user_model;
use crate::part::image::ImagePool;
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
use crate::result::{ExpectedVariant, RegularError, RegularResult};
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
    token: user_model::Token,
    data: member_data::CreateData,
) -> RegularResult<member_data::CreateVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
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

    MemberPermComplex::can_user_create(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.team_id,
    )
    .await?;

    let member_id = drive
        .with_context(async move |context| -> RegularResult<String> {
            //
            let repo = repo.derive_transactional().await;

            let user_info = repo
                .advance(context, &UserStep::get_info_excluded(&data.user_id))
                .await?;

            repo.advance(context, &TeamStep::get_info_excluded(&data.team_id))
                .await?;

            let existing_member_info = repo
                .advance(
                    context,
                    &MemberStep::find_info_by_user_id_and_team_id(
                        &data.user_id,
                        &data.team_id,
                    ),
                )
                .await?;

            if existing_member_info.is_some() {
                return Err(RegularError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-already-team-member"),
                });
            }

            let member_form = member_model::Form {
                id: MemberComplex::gen_id(),
                user_id: data.user_id,
                user_nickname: user_info.nickname,
                team_id: data.team_id,
                roles,
            };

            let member_info = repo
                .advance(context, &MemberStep::create(&member_form))
                .await?;

            Ok(member_info.id)
        })
        .await?;

    Ok(member_data::CreateVal { id: member_id })
}

/// Joins the current user to a team with a pending invitation code.
pub async fn join_team<D, C, R, I>(
    drive: &D,
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: member_data::JoinTeamData,
) -> RegularResult<member_data::InfoVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: MemberRepo<C> + MemberInvitationRepo<C> + UserRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberRepoTransactional<C>
            + MemberInvitationRepoTransactional<C>
            + UserRepoTransactional<C>
            + Send
            + Sync,
    I: ImagePool,
{
    let current_user_id = token.user_id;

    let member_info = drive
        .with_context(
            async move |context| -> RegularResult<member_model::Info> {
                //
                let repo = repo.derive_transactional().await;

                let current_user_info = repo
                    .advance(
                        context,
                        &UserStep::get_info_excluded(&current_user_id),
                    )
                    .await?;

                let member_invitation_info = repo
                    .advance(
                        context,
                        &MemberInvitationStep::get_info_by_code_excluded(
                            &data.code,
                        ),
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

                let member_form = member_model::Form {
                    id: MemberComplex::gen_id(),
                    user_id: current_user_id,
                    user_nickname: current_user_info.nickname,
                    team_id: member_invitation_info.team_id.clone(),
                    roles: member_invitation_info.roles,
                };

                let member_info = repo
                    .advance(context, &MemberStep::create(&member_form))
                    .await?;

                repo.advance(
                    context,
                    &MemberInvitationStep::mark_pending_as_used(
                        &member_invitation_info.id,
                    ),
                )
                .await?;

                Ok(member_info)
            },
        )
        .await?;

    member_data::InfoVal::from_model(image_pool, member_info).await
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: member_data::ListInfosData,
) -> RegularResult<Vec<member_data::InfoVal>>
where
    R: MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: MemberRepoTransactional<C>,
    I: ImagePool,
{
    let member_list_spec: member_model::ListSpec = data.try_into()?;

    //
    if let member_model::ListSpec::Team { team_id, .. } = &member_list_spec {
        //
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        MemberPermComplex::can_user_list_infos(
            &mut repo.as_proxy(),
            &token.user_id,
            team_id,
        )
        .await?;
    }

    let member_infos = repo
        .execute(&MemberStep::list_infos(&member_list_spec))
        .await?;

    let mut member_info_vals = Vec::with_capacity(member_infos.len());

    for member_info in member_infos {
        member_info_vals.push(
            member_data::InfoVal::from_model(image_pool, member_info).await?,
        );
    }

    Ok(member_info_vals)
}

/// Updates one member's roles.
///
/// The caller must be a team admin of the target member's team.
pub async fn update_roles<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    data: member_data::UpdateRolesData,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberRepoTransactional<C> + Send + Sync,
{
    let member_info = repo
        .execute(&MemberStep::get_info_by_id(&data.id, &[]))
        .await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberPermComplex::can_user_update_info(
        &mut repo.as_proxy(),
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            let member_role_update = member_model::RoleUpdate {
                id: data.id,
                roles: data.roles,
            };

            repo.advance(
                context,
                &MemberStep::update_role(&member_role_update),
            )
            .await?;

            Ok(())
        })
        .await?;

    Ok(())
}

/// Deletes one member.
///
/// The caller must be a team admin of the target member's team.
pub async fn delete<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        MemberRepoTransactional<C> + Send + Sync,
{
    let member_info =
        repo.execute(&MemberStep::get_info_by_id(&id, &[])).await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    MemberPermComplex::can_user_delete(
        &mut repo.as_proxy(),
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            repo.advance(context, &MemberStep::delete(&id)).await?;

            Ok(())
        })
        .await?;

    Ok(())
}

/// Constructs an args error for an invalid invitation code.
fn invalid_invitation_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-no-pending-invitation"),
    }
}

/// Constructs an args error for a user already in the team.
fn already_team_member_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-already-team-member"),
    }
}
