//! Assignment invitation use cases.

use std::time::Duration;

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _};
use poprako_orchestra_extra::prom::oper::Defer;
use poprako_orchestra_extra::prom::task::Task;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::ChapterComplex;
use crate::data::instr::assignment_invitation::{
    CreateAssignmentInvitationInstr, JoinAssignmentInvitationInstr,
    ListAssignmentInvitationInfosInstr,
};
use crate::data::val::assignment_invitation::CreateAssignmentInvitationVal;
use crate::data::view::assignment::AssignmentInfoView;
use crate::data::view::assignment_invitation::AssignmentInvitationInfoView;
use crate::model::read::spec::assignment_invitation::AssignmentInvitationListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::assignment_invitation::AssignmentInvitationEntry;
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
#[instrument(level = "info", skip(repo))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListAssignmentInvitationInfosInstr,
) -> BaseRest<Vec<AssignmentInvitationInfoView>>
where
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Sync,
{
    ensure_user_admin(repo, &token.user_id, &instr.chapter_id).await?;

    let assignment_invitation_list_spec = AssignmentInvitationListSpec {
        chapter_id: instr.chapter_id,
        is_pending: instr.is_pending,
        offset: instr.offset,
        limit: instr.limit,
    };

    let assignment_invitation_infos = ListAssignmentInvitationInfos {
        spec: &assignment_invitation_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(
        assignment_invitation_infos
            .into_iter()
            .map(AssignmentInvitationInfoView::from)
            .collect(),
    )
}

/// Creates a pending assignment invitation.
#[instrument(level = "info", skip(nucl, repo, prom))]
pub async fn create<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    instr: CreateAssignmentInvitationInstr,
) -> BaseRest<CreateAssignmentInvitationVal>
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
    validate_roles(instr.roles, &token.user_id, &instr.chapter_id)?;

    ensure_user_admin(repo, &token.user_id, &instr.chapter_id).await?;

    let (assignment_invitation_id, code) = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let invitee_user_info = FindUserInfo::Qid {
                qid: &instr.invitee_qid,
            }
            .step_on(repo, context)
            .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //

                let existing_assignment_info =
                    FindAssignmentInfo::ChapterUser {
                        chapter_id: &instr.chapter_id,
                        user_id: &invitee_user_info.id,
                    }
                    .step_on(repo, context)
                    .await?;

                if existing_assignment_info.is_some() {
                    //
                    let err_message = trl("error-assignment-already-exists");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Args,
                        err_message = %err_message,
                        chapter_id = %instr.chapter_id,
                        user_id = %token.user_id,
                        invitee_user_id = %invitee_user_info.id,
                        invitee_qid = %instr.invitee_qid,
                        roles = ?instr.roles,
                        "expected error: invitee already has a chapter assignment",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Args,
                        message: err_message,
                    });
                }
            }

            let (assignment_invitation_id, code) =
                (gen_assignment_invitation_id(), gen_code());

            let assignment_invitation_entry = AssignmentInvitationEntry {
                id: assignment_invitation_id,
                chapter_id: instr.chapter_id,
                inviter_id: token.user_id,
                invitee_qid: instr.invitee_qid,
                code,
                roles: instr.roles,
            };

            let assignment_invitation_info = CreateAssignmentInvitation {
                entry: &assignment_invitation_entry,
            }
            .step_on(repo, context)
            .await?;

            let purge_event = InvitationPayload::Assignment {
                invitation_id: assignment_invitation_info.id.clone(),
            };

            let (purge_payload, purge_task_id) =
                (TaskPayload::Invitation(purge_event), next_snowflake_id());

            let purge_task = Task {
                id: &purge_task_id,
                payload: &purge_payload,
                delay: Some(EXPIRY_DELAY),
            };

            Defer::new(purge_task).step_on(prom, context).await?;

            accept((
                assignment_invitation_info.id,
                assignment_invitation_info.code,
            ))
        })
        .await?;

    accept(CreateAssignmentInvitationVal {
        id: assignment_invitation_id,
        code,
    })
}

/// Deletes an assignment invitation.
#[instrument(level = "info", skip(nucl, repo))]
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
    let assignment_invitation_info =
        GetAssignmentInvitationInfo::Id { id: &id }
            .run_on(repo)
            .await?;

    ensure_user_admin(
        repo,
        &token.user_id,
        &assignment_invitation_info.chapter_id,
    )
    .await?;

    nucl.coord(async move |context| {
        //

        DeleteAssignmentInvitations::Id { id: &id }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Joins a chapter assignment with a pending invitation code.
#[instrument(
    level = "info",
    skip(nucl, repo, image_pool, instr),
    fields(code = "[REDACTED]")
)]
pub async fn join<N, C, R, I>(
    (nucl, repo, image_pool): (&N, &R, &I),
    token: UserToken,
    instr: JoinAssignmentInvitationInstr,
) -> BaseRest<AssignmentInfoView>
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

            let current_user_info = GetUserInfoExcluded::Id {
                id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_invitation_info =
                GetAssignmentInvitationInfoExcluded { code: &instr.code }
                    .step_on(repo, context)
                    .await?;

            if assignment_invitation_info.invitee_qid != current_user_info.qid {
                //
                let err_message = trl("error-no-pending-invitation");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Args,
                    err_message = %err_message,
                    user_id = %current_user_id,
                    invitee_qid = %current_user_info.qid,
                    invitation_invitee_qid = %assignment_invitation_info.invitee_qid,
                    invitation_code_present = true,
                    "expected error: assignment invitation does not belong to current user",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: err_message,
                });
            }

            validate_roles(
                assignment_invitation_info.roles,
                &current_user_id,
                &assignment_invitation_info.chapter_id,
            )?;

            let chapter_info = GetChapterInfoExcluded {
                id: &assignment_invitation_info.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let comic_info = GetComicInfo {
                id: &chapter_info.comic_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            let workset_info = GetWorksetInfo {
                id: &comic_info.workset_id,
            }
            .step_on(repo, context)
            .await?;

            let member_info = FindMemberInfo::UserTeam {
                user_id: &current_user_id,
                team_id: &workset_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            let Some(member_info) = member_info else {
                //
                let err_message = trl("error-chapter-role-not-assignable");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    chapter_id = %assignment_invitation_info.chapter_id,
                    user_id = %current_user_id,
                    team_id = %workset_info.team_id,
                    roles = ?assignment_invitation_info.roles,
                    "expected error: invited chapter roles are not assignable",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            };

            if !member_info
                .roles
                .contains_mask(assignment_invitation_info.roles)
            {
                let err_message = trl("error-chapter-role-not-assignable");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    chapter_id = %assignment_invitation_info.chapter_id,
                    user_id = %current_user_id,
                    team_id = %workset_info.team_id,
                    roles = ?assignment_invitation_info.roles,
                    member_roles = ?member_info.roles,
                    "expected error: invited chapter roles are not assignable",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            }

            let existing_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &assignment_invitation_info.chapter_id,
                user_id: &current_user_id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_info = match existing_assignment_info {
                //
                Some(existing_assignment_info) => {
                    //
                    let assignment_role_update = AssignmentComplex::merge_roles(
                        &existing_assignment_info,
                        assignment_invitation_info.roles,
                    );

                    UpdateAssignmentRoles {
                        update: &assignment_role_update,
                    }
                    .step_on(repo, context)
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

                    CreateAssignment {
                        entry: &assignment_entry,
                    }
                    .step_on(repo, context)
                    .await?
                }
            };

            MarkAssignmentInvitationUsed {
                id: &assignment_invitation_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept(assignment_info)
        })
        .await?;

    AssignmentInfoView::from_model(image_pool, assignment_info, None).await
}

// Verifies that the current user is assigned as a chapter administrator.
#[instrument(level = "info", skip(repo))]
async fn ensure_user_admin<C, R>(
    repo: &R,
    current_user_id: &str,
    chapter_id: &str,
) -> BaseRest<()>
where
    R: AssignmentRepo<C>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: current_user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %current_user_id,
            "expected error: chapter administrator perm required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-admin-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %current_user_id,
            roles = ?assignment_info.roles,
            "expected error: chapter administrator perm required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Validates that the roles mask is non-empty and does not contain ADMIN.
fn validate_roles(
    roles: RoleMask,
    user_id: &str,
    chapter_id: &str,
) -> BaseRest<()> {
    //
    if u32::from(roles) == 0 || roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            chapter_id = %chapter_id,
            user_id = %user_id,
            roles = ?roles,
            "expected error: chapter roles are not assignable",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    accept(())
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
