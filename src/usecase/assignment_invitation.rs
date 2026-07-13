//! Assignment invitation use cases.

use poprako_orchestra::Nucl;

use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::complex::assignment::AssignmentComplex;
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationParams,
    CreateAssignmentInvitationPayload, JoinAssignmentInvitationParams,
    ListAssignmentInvitationInfosParams,
};
use crate::model::assignment::{AssignmentEntry, AssignmentInfo};
use crate::model::assignment_invitation::AssignmentInvitationEntry;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
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
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::{FindUserInfo, GetUserInfoExcluded};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::next_snowflake_id;
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
mod tests;

// FIXME: invitations should be fired out after a period of time.

/// Lists assignment invitations under one chapter.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    params: ListAssignmentInvitationInfosParams,
) -> RegularResult<Vec<AssignmentInvitationInfoVal>>
where
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Sync,
{
    ensure_user_admin(repo, &token.user_id, &params.chapter_id).await?;

    let assignment_invitation_infos = repo
        .run(&ListAssignmentInvitationInfos {
            chapter_id: &params.chapter_id,
            pending: params.pending,
            page: Page {
                offset: params.offset,
                limit: params.limit,
            },
        })
        .await?;

    Ok(assignment_invitation_infos
        .into_iter()
        .map(AssignmentInvitationInfoVal::from)
        .collect())
}

/// Creates a pending assignment invitation.
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateAssignmentInvitationParams,
) -> RegularResult<CreateAssignmentInvitationPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
{
    validate_roles(params.roles)?;

    ensure_user_admin(repo, &token.user_id, &params.chapter_id).await?;

    let (assignment_invitation_id, code) = nucl
        .coord(async move |context| -> RegularResult<(String, String)> {
            //
            let find_user_info = FindUserInfo::Qid {
                qid: &params.invitee_qid,
            };

            let invitee_user_info = repo.step(context, &find_user_info).await?;

            if let Some(invitee_user_info) = invitee_user_info {
                //
                let find_assignment_info = FindAssignmentInfo::ChapterUser {
                    chapter_id: &params.chapter_id,
                    user_id: &invitee_user_info.id,
                };

                let existing_assignment_info =
                    repo.step(context, &find_assignment_info).await?;

                if existing_assignment_info.is_some() {
                    return Err(invitee_assigned_error());
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

            Ok((
                assignment_invitation_info.id,
                assignment_invitation_info.code,
            ))
        })
        .await?;

    Ok(CreateAssignmentInvitationPayload {
        id: assignment_invitation_id,
        code,
    })
}

/// Deletes an assignment invitation.
pub async fn delete<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    let get_assignment_invitation_info =
        GetAssignmentInvitationInfo::Id { id: &id };

    let assignment_invitation_info =
        repo.run(&get_assignment_invitation_info).await?;

    ensure_user_admin(
        repo,
        &token.user_id,
        &assignment_invitation_info.chapter_id,
    )
    .await?;

    nucl.coord(async move |context| -> RegularResult<()> {
        //
        let delete_assignment_invitation =
            DeleteAssignmentInvitations::Id { id: &id };

        repo.step(context, &delete_assignment_invitation).await?;

        Ok(())
    })
    .await?;

    Ok(())
}

/// Joins a chapter assignment with a pending invitation code.
pub async fn join<N, C, R, I>(
    nucl: &N,
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: JoinAssignmentInvitationParams,
) -> RegularResult<AssignmentInfoVal>
where
    N: Nucl<Context = C, Error = RegularError>,
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
        .coord(async move |context| -> RegularResult<AssignmentInfo> {
            //
            let get_user_info_excluded = GetUserInfoExcluded::Id {
                id: &current_user_id,
            };

            let current_user_info =
                repo.step(context, &get_user_info_excluded).await?;

            let assignment_invitation_info = repo
                .step(
                    context,
                    &GetAssignmentInvitationInfoExcluded { code: &params.code },
                )
                .await?;

            if assignment_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_error());
            }

            validate_roles(assignment_invitation_info.roles)?;

            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfo {
                        id: &assignment_invitation_info.chapter_id,
                        incls: &[],
                    },
                )
                .await?;

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

            let find_member_info = FindMemberInfo::UserTeam {
                user_id: &current_user_id,
                team_id: &workset_info.team_id,
            };

            let member_info = repo.step(context, &find_member_info).await?;

            let Some(member_info) = member_info else {
                return Err(assignment_role_not_assignable_perm_error());
            };

            if !member_info
                .roles
                .contains_mask(assignment_invitation_info.roles)
            {
                return Err(assignment_role_not_assignable_perm_error());
            }

            let find_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &assignment_invitation_info.chapter_id,
                user_id: &current_user_id,
            };

            let existing_assignment_info =
                repo.step(context, &find_assignment_info).await?;

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

            Ok(assignment_info)
        })
        .await?;

    AssignmentInfoVal::from_model(image_pool, assignment_info).await
}

/// Verifies that the current user is assigned as a chapter administrator.
async fn ensure_user_admin<C, R>(
    repo: &R,
    current_user_id: &str,
    chapter_id: &str,
) -> RegularResult<()>
where
    R: AssignmentRepo<C>,
{
    let find_assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id: current_user_id,
    };

    let assignment_info = repo.run(&find_assignment_info).await?;

    let Some(assignment_info) = assignment_info else {
        return Err(chapter_admin_error());
    };

    if !assignment_info.roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(chapter_admin_error());
    }

    Ok(())
}

/// Generates a snowflake ID for a new invitation.
fn gen_assignment_invitation_id() -> String {
    next_snowflake_id()
}

/// Generates a short numeric code from a snowflake ID.
fn gen_code() -> String {
    //
    let id = next_snowflake_id();

    let len = id.len();

    if len <= 6 {
        return id;
    }

    id[len - 6..].to_string()
}

/// Validates that the roles mask is non-empty and does not contain ADMIN.
fn validate_roles(roles: RoleMask) -> RegularResult<()> {
    //
    if u32::from(roles) == 0 || roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(assignment_role_not_assignable_args_error());
    }

    Ok(())
}

/// Constructs an args error for an invalid invitation code.
fn invalid_invitation_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-no-pending-invitation"),
    }
}

/// Constructs an args error for an already assigned invitee.
fn invitee_assigned_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-assignment-already-exists"),
    }
}

/// Constructs a permission error when the caller is not a chapter admin.
fn chapter_admin_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-admin-required"),
    }
}

/// Constructs an args error for unassignable chapter roles.
fn assignment_role_not_assignable_args_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}

/// Constructs a permission error for unassignable chapter roles.
fn assignment_role_not_assignable_perm_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-role-not-assignable"),
    }
}
