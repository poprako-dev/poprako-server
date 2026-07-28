//! Member use cases: create, join, list, role update, and deletion.

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::member::{MemberComplex, MemberPermComplex};
use crate::data::member::{
    CreateMemberParams, CreateMemberPayload, JoinTeamParams,
    ListMemberInfosParams, MemberInfoVal, UpdateMemberRolesParams,
};
use crate::model::read::spec::member::MemberListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member::{MemberEntry, MemberRoleRepl};
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
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::user::GetUserInfoExcluded;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

#[cfg(test)]
// Unit tests for team membership and invitation boundary conditions.
mod tests;

/// Creates one member under a team.
///
/// The caller must be a team admin. The target user and team are locked in
/// the transaction before inserting the membership.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: CreateMemberParams,
) -> BaseRest<CreateMemberPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
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
        .coord(async move |context| {
            //

            let user_info = GetUserInfoExcluded::Id {
                id: &params.user_id,
            }
            .step_on(repo, context)
            .await?;

            LockTeam {
                id: &params.team_id,
            }
            .step_on(repo, context)
            .await?;

            let existing_member_info = FindMemberInfo::UserTeam {
                user_id: &params.user_id,
                team_id: &params.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                return Err(BaseError::Expected {
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

            let member_info = CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(member_info.id)
        })
        .await?;

    accept(CreateMemberPayload { id: member_id })
}

/// Joins the current user to a team with a pending invitation code.
#[instrument(
    level = "info",
    err(Debug),
    skip(nucl, repo, image_pool, params),
    fields(code = "[REDACTED]")
)]
pub async fn join_team<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    params: JoinTeamParams,
) -> BaseRest<MemberInfoVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
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
                GetMemberInvitationInfoExcluded::Code { code: &params.code }
                    .step_on(repo, context)
                    .await?;

            if member_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_err());
            }

            let existing_member_info = FindMemberInfo::UserTeam {
                user_id: &current_user_id,
                team_id: &member_invitation_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if existing_member_info.is_some() {
                return Err(already_team_member_err());
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

    MemberInfoVal::from_model(image_pool, member_info).await
}

/// Lists members under one team.
///
/// The caller must already be a member of the target team.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    params: ListMemberInfosParams,
) -> BaseRest<Vec<MemberInfoVal>>
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

    let member_infos = ListMemberInfos::Spec {
        spec: &member_list_spec,
    }
    .run_on(repo)
    .await?;

    let mut member_info_vals = Vec::with_capacity(member_infos.len());

    for member_info in member_infos {
        member_info_vals
            .push(MemberInfoVal::from_model(image_pool, member_info).await?);
    }

    accept(member_info_vals)
}

/// Updates one member's roles.
///
/// The caller must be a team admin of the target member's team.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: UpdateMemberRolesParams,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = GetMemberInfo::Id {
        id: &params.id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    MemberPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let member_role_update = MemberRoleRepl {
            id: params.id,
            roles: params.roles,
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
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: MemberRepo<C> + Send + Sync,
{
    let member_info = GetMemberInfo::Id {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    MemberPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &member_info.team_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        DeleteMember { id: &id }.step_on(repo, context).await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

// Constructs an args error for an invalid invitation code.
fn invalid_invitation_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-no-pending-invitation"),
    }
}

// Constructs an args error for a user already in the team.
fn already_team_member_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-already-team-member"),
    }
}
