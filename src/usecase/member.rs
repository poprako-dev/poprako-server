//! Member use cases: create, join, list, role update, and deletion.

use tracing::instrument;

use poprako_orchestra::{Nucl, run_proxy};

use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member::{
    CreateMemberParams, CreateMemberPayload, JoinTeamParams,
    ListMemberInfosParams, MemberInfoVal, UpdateMemberRolesParams,
};
use crate::model::member::{
    MemberEntry, MemberInfo, MemberListSpec, MemberRoleUpdate,
};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    UpdateMember,
};
use crate::part::repo::oper::member_invitation::{
    GetMemberInvitationInfoExcluded, UpdateMemberInvitation,
};
use crate::part::repo::oper::team::GetTeamInfoExcluded;
use crate::part::repo::oper::user::GetUserInfoExcluded;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

#[cfg(test)]
mod tests;

/// Creates one member under a team.
///
/// The caller must be a team admin. The target user and team are locked in
/// the transaction before inserting the membership.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateMemberParams,
) -> RegularResult<CreateMemberPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: MemberRepo<C> + TeamRepo<C> + UserRepo<C> + Send + Sync,
{
    let roles = params.roles;

    MemberPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let member_id = nucl
        .coord(async move |context| -> RegularResult<String> {
            //

            let user_info = repo
                .step(
                    context,
                    &GetUserInfoExcluded::Id {
                        id: &params.user_id,
                    },
                )
                .await?;

            repo.step(
                context,
                &GetTeamInfoExcluded::Id {
                    id: &params.team_id,
                },
            )
            .await?;

            let existing_member_info = repo
                .step(
                    context,
                    &FindMemberInfo::UserTeam {
                        user_id: &params.user_id,
                        team_id: &params.team_id,
                    },
                )
                .await?;

            if existing_member_info.is_some() {
                return Err(RegularError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-already-team-member"),
                });
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: params.user_id,
                user_nickname: user_info.nickname,
                team_id: params.team_id,
                roles,
            };

            let member_info = repo
                .step(
                    context,
                    &CreateMember {
                        entry: &member_entry,
                    },
                )
                .await?;

            Ok(member_info.id)
        })
        .await?;

    Ok(CreateMemberPayload { id: member_id })
}

/// Joins the current user to a team with a pending invitation code.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn join_team<N, C, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: JoinTeamParams,
) -> RegularResult<MemberInfoVal>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: MemberRepo<C> + MemberInvitationRepo<C> + UserRepo<C> + Send + Sync,
    I: ImagePool,
{
    let current_user_id = token.user_id;

    let member_info = nucl
        .coord(async move |context| -> RegularResult<MemberInfo> {
            //

            let current_user_info = repo
                .step(
                    context,
                    &GetUserInfoExcluded::Id {
                        id: &current_user_id,
                    },
                )
                .await?;

            let member_invitation_info = repo
                .step(
                    context,
                    &GetMemberInvitationInfoExcluded::Code {
                        code: &params.code,
                    },
                )
                .await?;

            if member_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_error());
            }

            let existing_member_info = repo
                .step(
                    context,
                    &FindMemberInfo::UserTeam {
                        user_id: &current_user_id,
                        team_id: &member_invitation_info.team_id,
                    },
                )
                .await?;

            if existing_member_info.is_some() {
                return Err(already_team_member_error());
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: current_user_id,
                user_nickname: current_user_info.nickname,
                team_id: member_invitation_info.team_id.clone(),
                roles: member_invitation_info.roles,
            };

            let member_info = repo
                .step(
                    context,
                    &CreateMember {
                        entry: &member_entry,
                    },
                )
                .await?;

            repo.step(
                context,
                &UpdateMemberInvitation::MarkUsed {
                    id: &member_invitation_info.id,
                },
            )
            .await?;

            Ok(member_info)
        })
        .await?;

    MemberInfoVal::from_model(image_pool, member_info).await
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListMemberInfosParams,
) -> RegularResult<Vec<MemberInfoVal>>
where
    R: MemberRepo<C> + Sync,
    I: ImagePool,
{
    let member_list_spec: MemberListSpec = params.try_into()?;

    if let MemberListSpec::Team { team_id, .. } = &member_list_spec {
        MemberPermComplex::ensure_user_can_list_infos(
            &mut run_proxy! {
                repo => for<'a> FindMemberInfo<'a>;
            },
            &token.user_id,
            team_id,
        )
        .await?;
    }

    let member_infos = repo
        .run(&ListMemberInfos::Spec {
            spec: &member_list_spec,
        })
        .await?;

    let mut member_info_vals = Vec::with_capacity(member_infos.len());

    for member_info in member_infos {
        member_info_vals
            .push(MemberInfoVal::from_model(image_pool, member_info).await?);
    }

    Ok(member_info_vals)
}

/// Updates one member's roles.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_roles<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateMemberRolesParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = repo
        .run(&GetMemberInfo::Id {
            id: &params.id,
            incls: &[],
        })
        .await?;

    MemberPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    nucl
        .coord(async move |context| -> RegularResult<()> {
            //
            let member_role_update = MemberRoleUpdate {
                id: params.id,
                roles: params.roles,
            };

            repo.step(
                context,
                &UpdateMember::Role {
                    update: &member_role_update,
                },
            )
            .await?;

            Ok(())
        })
        .await?;

    let () = ();

    Ok(())
}

/// Deletes one member.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = repo
        .run(&GetMemberInfo::Id {
            id: &id,
            incls: &[],
        })
        .await?;

    MemberPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    nucl
        .coord(async move |context| -> RegularResult<()> {
            //
            repo.step(context, &DeleteMember { id: &id }).await?;

            Ok(())
        })
        .await?;

    let () = ();

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
