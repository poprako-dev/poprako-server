//! Assignment invitation use cases.

use std::time::Duration;

use poprako_orchestra::Nucl;
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationParams,
    CreateAssignmentInvitationPayload, JoinAssignmentInvitationParams,
    ListAssignmentInvitationInfosParams,
};
use crate::model::assignment::AssignmentEntry;
use crate::model::assignment_invitation::{
    AssignmentInvitationEntry, AssignmentInvitationListKind,
    AssignmentInvitationListSpec,
};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::TaskPayload;
use crate::part::prom::payload::invitation::InvitationPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo, UpdateAssignmentRoles,
};
use crate::part::repo::oper::assignment_invitation::{
    CreateAssignmentInvitation, DeleteAssignmentInvitations,
    GetAssignmentInvitationInfo, GetAssignmentInvitationInfoExcluded,
    ListAssignmentInvitationInfos, MarkAssignmentInvitationUsed,
};
use crate::part::repo::oper::chapter::GetChapterInfoExcluded;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::{FindUserInfo, GetUserInfoExcluded};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
// Unit tests for assignment invitation acceptance rules.
mod tests;

// Assignment invitation code expiration window.
const EXPIRY_DELAY: Duration = Duration::from_secs(3 * 24 * 60 * 60);

/// Lists assignment invitations under one chapter.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: ListAssignmentInvitationInfosParams,
) -> BaseRest<Vec<AssignmentInvitationInfoVal>>
where
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Sync,
{
    ensure_user_admin(repo, &token.user_id, &params.chapter_id).await?;

    let kind = match params.pending {
        //
        Some(true) => AssignmentInvitationListKind::Pending,

        Some(false) => AssignmentInvitationListKind::Used,

        None => AssignmentInvitationListKind::All,
    };

    let assignment_invitation_list_spec = AssignmentInvitationListSpec {
        chapter_id: params.chapter_id,
        kind,
        offset: params.offset,
        limit: params.limit,
    };

    let assignment_invitation_infos = repo
        .run(&ListAssignmentInvitationInfos {
            spec: &assignment_invitation_list_spec,
        })
        .await?;

    accept(
        assignment_invitation_infos
            .into_iter()
            .map(AssignmentInvitationInfoVal::from)
            .collect(),
    )
}

/// Creates a pending assignment invitation.
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom))]
pub async fn create<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    params: CreateAssignmentInvitationParams,
) -> BaseRest<CreateAssignmentInvitationPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + ChapterRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    validate_roles(params.roles)?;

    ensure_user_admin(repo, &token.user_id, &params.chapter_id).await?;

    let (assignment_invitation_id, code) = nucl
        .coord(async move |context| {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &params.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let invitee_user_info = repo
                .step(
                    context,
                    &FindUserInfo::Qid {
                        qid: &params.invitee_qid,
                    },
                )
                .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //

                let existing_assignment_info = repo
                    .step(
                        context,
                        &FindAssignmentInfo::ChapterUser {
                            chapter_id: &params.chapter_id,
                            user_id: &invitee_user_info.id,
                        },
                    )
                    .await?;

                if existing_assignment_info.is_some() {
                    return Err(invitee_assigned_err());
                }
            }

            let assignment_invitation_id = gen_assignment_invitation_id();

            let code = gen_code();

            let assignment_invitation_entry = AssignmentInvitationEntry {
                id: assignment_invitation_id,
                chapter_id: params.chapter_id,
                inviter_id: token.user_id,
                invitee_qid: params.invitee_qid,
                code,
                roles: params.roles,
            };

            let assignment_invitation_info = repo
                .step(
                    context,
                    &CreateAssignmentInvitation {
                        entry: &assignment_invitation_entry,
                    },
                )
                .await?;

            let purge_event = InvitationPayload::Assignment {
                invitation_id: assignment_invitation_info.id.clone(),
            };

            let purge_payload = TaskPayload::Invitation(purge_event);

            let purge_task_id = next_snowflake_id();

            let purge_task = Task {
                id: &purge_task_id,
                payload: &purge_payload,
                delay: Some(EXPIRY_DELAY),
            };

            prom.step(context, &Defer::new(purge_task)).await?;

            accept((
                assignment_invitation_info.id,
                assignment_invitation_info.code,
            ))
        })
        .await?;

    accept(CreateAssignmentInvitationPayload {
        id: assignment_invitation_id,
        code,
    })
}

/// Deletes an assignment invitation.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    let assignment_invitation_info = repo
        .run(&GetAssignmentInvitationInfo::Id { id: &id })
        .await?;

    ensure_user_admin(
        repo,
        &token.user_id,
        &assignment_invitation_info.chapter_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //

        repo.step(context, &DeleteAssignmentInvitations::Id { id: &id })
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Joins a chapter assignment with a pending invitation code.
#[instrument(
    level = "info",
    err(Debug),
    skip(nucl, repo, image_pool, params),
    fields(code = "[REDACTED]")
)]
pub async fn join<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    params: JoinAssignmentInvitationParams,
) -> BaseRest<AssignmentInfoVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UserRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + Send
        + Sync,
    I: ImagePool,
{
    let current_user_id = token.user_id;

    let assignment_info = nucl
        .coord(async move |context| {
            //

            let current_user_info = repo
                .step(
                    context,
                    &GetUserInfoExcluded::Id {
                        id: &current_user_id,
                    },
                )
                .await?;

            let assignment_invitation_info = repo
                .step(
                    context,
                    &GetAssignmentInvitationInfoExcluded { code: &params.code },
                )
                .await?;

            if assignment_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_err());
            }

            validate_roles(assignment_invitation_info.roles)?;

            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &assignment_invitation_info.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let comic_info = repo
                .step(
                    context,
                    &GetComicInfo {
                        id: &chapter_info.comic_id,
                        incls: &[],
                    },
                )
                .await?;

            let workset_info = repo
                .step(
                    context,
                    &GetWorksetInfo {
                        id: &comic_info.workset_id,
                    },
                )
                .await?;

            let member_info = repo
                .step(
                    context,
                    &FindMemberInfo::UserTeam {
                        user_id: &current_user_id,
                        team_id: &workset_info.team_id,
                    },
                )
                .await?;

            let Some(member_info) = member_info else {
                return Err(assignment_role_not_assignable_perm_err());
            };

            if !member_info
                .roles
                .contains_mask(assignment_invitation_info.roles)
            {
                return Err(assignment_role_not_assignable_perm_err());
            }

            let existing_assignment_info = repo
                .step(
                    context,
                    &FindAssignmentInfo::ChapterUser {
                        chapter_id: &assignment_invitation_info.chapter_id,
                        user_id: &current_user_id,
                    },
                )
                .await?;

            let assignment_info = match existing_assignment_info {
                //
                Some(existing_assignment_info) => {
                    //
                    let assignment_role_update = AssignmentComplex::merge_roles(
                        &existing_assignment_info,
                        assignment_invitation_info.roles,
                    );

                    repo.step(
                        context,
                        &UpdateAssignmentRoles {
                            update: &assignment_role_update,
                        },
                    )
                    .await?
                }

                None => {
                    //
                    let assignment_entry = AssignmentEntry {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: assignment_invitation_info
                            .chapter_id
                            .clone(),
                        user_id: current_user_id,
                        roles: assignment_invitation_info.roles,
                    };

                    repo.step(
                        context,
                        &CreateAssignment {
                            entry: &assignment_entry,
                        },
                    )
                    .await?
                }
            };

            repo.step(
                context,
                &MarkAssignmentInvitationUsed {
                    id: &assignment_invitation_info.id,
                },
            )
            .await?;

            accept(assignment_info)
        })
        .await?;

    AssignmentInfoVal::from_model(image_pool, assignment_info, None).await
}

// Verifies that the current user is assigned as a chapter administrator.
#[instrument(level = "info", err(Debug), skip(repo))]
async fn ensure_user_admin<C, R>(
    repo: &R,
    current_user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    R: AssignmentRepo<C>,
{
    let assignment_info = repo
        .run(&FindAssignmentInfo::ChapterUser {
            chapter_id,
            user_id: current_user_id,
        })
        .await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_admin_err());
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_admin_err());
    }

    accept(())
}

// Validates that the roles mask is non-empty and does not contain ADMIN.
fn validate_roles(roles: RoleMask) -> BaseRest<()> {
    //
    if u32::from(roles) == 0 || roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(assignment_role_not_assignable_args_err());
    }

    accept(())
}

// Constructs an args error for an already assigned invitee.
fn invitee_assigned_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-assignment-already-exists"),
    }
}

// Generates a snowflake ID for a new invitation.
fn gen_assignment_invitation_id() -> String {
    next_snowflake_id()
}

// Generates a short numeric code from a snowflake ID.
fn gen_code() -> String {
    //
    let id = next_snowflake_id();

    let len = id.len();

    if len <= 6 {
        return id;
    }

    id[len - 6..].to_string()
}

// Constructs an args error for an invalid invitation code.
fn invalid_invitation_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-no-pending-invitation"),
    }
}

// Constructs a permission error for unassignable chapter roles.
fn assignment_role_not_assignable_perm_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-role-not-assignable"),
    }
}

// Constructs a permission error when the caller is not a chapter admin.
fn chapter_admin_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-admin-required"),
    }
}

// Constructs an args error for unassignable chapter roles.
fn assignment_role_not_assignable_args_err() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}
