//! Member use cases: create, join, list, role update, and deletion.

#[cfg(test)]
// Unit tests for team membership and invitation boundary conditions.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::instr::member::{
    CreateMemberInstr, JoinTeamInstr, ListMemberInfosInstr,
    UpdateMemberRolesInstr,
};
use crate::data::val::member::CreateMemberVal;
use crate::data::view::member::MemberInfoView;
use crate::model::read::spec::member::MemberListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member::{MemberEntry, MemberRoleRepl};
use crate::part::image::ImagePool;
use crate::part::nucl::ReptRead;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::{
    CreateMember, DeleteMember, FindMemberInfo, GetMemberInfo, ListMemberInfos,
    UpdateMember,
};
use crate::part::repo::oper::member_invitation::{
    GetMemberInvitationInfoExcluded, UpdateMemberInvitation,
};
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::user::GetUserInfoExcluded;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Creates one member under a team.
///
/// The caller must be a team admin. The target user and team are locked in
/// the transaction before inserting the membership.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateMemberInstr,
) -> BaseRest<CreateMemberVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C> + TeamRepo<C> + UserRepo<C> + Send + Sync,
{
    let roles = instr.roles;

    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &instr.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    MemberPermComplex::ensure_user_can_create(&member_info)?;

    let member_id = nucl
        .coord(async move |context| {
            //

            let user_info = GetUserInfoExcluded::Id { id: &instr.user_id }
                .step_on(repo, context)
                .await?;

            LockTeam { id: &instr.team_id }
                .step_on(repo, context)
                .await?;

            let existing_member_info = FindMemberInfo::UserTeam {
                user_id: &instr.user_id,
                team_id: &instr.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                //
                let err_message = trl("error-already-team-member");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    team_id = %instr.team_id,
                    user_id = %token.user_id,
                    affected_user_id = %instr.user_id,
                    roles = ?roles,
                    "expected error: user is already a team member",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: instr.user_id,
                user_nickname: user_info.nickname,
                team_id: instr.team_id,
                roles,
            };

            let member_info = CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(member_info.id)
        })
        .await?;

    accept(CreateMemberVal { id: member_id })
}

/// Joins the current user to a team with a pending invitation code.
#[instrument(
    level = "info",
    skip(nucl, repo, image_pool, instr),
    fields(code = "[REDACTED]")
)]
pub async fn join_team<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    instr: JoinTeamInstr,
) -> BaseRest<MemberInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C> + MemberInvitationRepo<C> + UserRepo<C> + Send + Sync,
    I: ImagePool,
{
    let current_user_id = token.user_id;

    let member_info = nucl
        .coord(async move |context| {
            //

            let current_user_info = GetUserInfoExcluded::Id {
                id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let member_invitation_info =
                GetMemberInvitationInfoExcluded::Code { code: &instr.code }
                    .step_on(repo, context)
                    .await?;

            if member_invitation_info.invitee_qid != current_user_info.qid {
                //
                let err_message = trl("error-no-pending-invitation");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    user_id = %current_user_id,
                    invitee_qid = %current_user_info.qid,
                    invitation_invitee_qid = %member_invitation_info.invitee_qid,
                    team_id = %member_invitation_info.team_id,
                    "expected error: invitation does not belong to current user",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let existing_member_info = FindMemberInfo::UserTeam {
                user_id: &current_user_id,
                team_id: &member_invitation_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                //
                let err_message = trl("error-already-team-member");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    user_id = %current_user_id,
                    team_id = %member_invitation_info.team_id,
                    "expected error: user is already a team member",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: current_user_id,
                user_nickname: current_user_info.nickname,
                team_id: member_invitation_info.team_id.clone(),
                roles: member_invitation_info.roles,
            };

            let member_info = CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateMemberInvitation::MarkUsed {
                id: &member_invitation_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept(member_info)
        })
        .await?;

    MemberInfoView::from_model(image_pool, member_info).await
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListMemberInfosInstr,
) -> BaseRest<Vec<MemberInfoView>>
where
    C: Context,
    R: MemberRepo<C> + Sync,
    I: ImagePool,
{
    let member_list_spec = instr.try_into()?;

    if let MemberListSpec::Team { team_id, .. } = &member_list_spec {
        //
        let member_info = FindMemberInfo::UserTeam {
            user_id: &token.user_id,
            team_id,
        }
        .run_on(repo)
        .await?;

        let Some(member_info) = member_info else {
            //
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-team-member-required"),
            });
        };

        MemberPermComplex::ensure_user_can_list_infos(&member_info)?;
    }

    let member_infos = ListMemberInfos::Spec {
        spec: &member_list_spec,
    }
    .run_on(repo)
    .await?;

    let mut member_info_vals = Vec::with_capacity(member_infos.len());

    for member_info in member_infos {
        //
        member_info_vals
            .push(MemberInfoView::from_model(image_pool, member_info).await?);
    }

    accept(member_info_vals)
}

/// Updates one member's roles.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateMemberRolesInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = GetMemberInfo::Id {
        id: &instr.id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let caller_member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &member_info.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(caller_member_info) = caller_member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    MemberPermComplex::ensure_user_can_update_info(&caller_member_info)?;

    nucl.coord(async move |context| {
        //
        let member_role_update = MemberRoleRepl {
            id: instr.id,
            roles: instr.roles,
        };

        UpdateMember::Role {
            update: &member_role_update,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

/// Deletes one member.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = GetMemberInfo::Id {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let caller_member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &member_info.team_id,
    }
    .run_on(repo)
    .await?;

    let Some(caller_member_info) = caller_member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    MemberPermComplex::ensure_user_can_delete(&caller_member_info)?;

    nucl.coord(async move |context| {
        //
        DeleteMember { id: &id }.step_on(repo, context).await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}
