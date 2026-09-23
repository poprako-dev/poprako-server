//! Assignment use cases — list, join, role update, and deletion.

/// Assignment role-update orchestration.
pub mod update_roles;

#[cfg(test)]
// Unit tests that cover assignment orchestration invariants.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDeptView;
use poprako_util::i18n::trl;

use crate::complex::assignment::perm as assignment_perm_complex;
use crate::complex::chapter::perm as chapter_perm_complex;
use crate::complex::{
    assignment as assignment_complex, chapter as chapter_complex,
};
use crate::data::instr::assignment::{
    JoinChapterAssignmentInstr, ListAssignmentInfosInstr,
};
use crate::data::view::assignment::AssignmentInfoView;
use crate::model::read::spec::assignment::AssignmentListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar, UserAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, UpdateAssignmentRoles,
};
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::usecase::internal::view::assignment_info_views;
use crate::value::chapter_workflow_record::ChapterWorkflowRecordPayload;

/// Lists assignments by chapter or owner user.
#[instrument(level = "info", skip(repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    instr: ListAssignmentInfosInstr,
) -> BaseRest<Vec<AssignmentInfoView>>
where
    C: Context,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + UserRepo<C>
        + PageRepo<C>
        + Sync,
    O: ObjDeptView<ComicCover, C>
        + ObjDeptView<PageImage, C>
        + ObjDeptView<TeamAvatar, C>
        + ObjDeptView<UserAvatar, C>
        + Sync,
{
    let assignment_list_spec = instr.try_into()?;

    ensure_user_can_list::<C, R>(repo, &token, &assignment_list_spec).await?;

    let assignment_infos = ListAssignmentInfos::Spec {
        spec: &assignment_list_spec,
    }
    .run_on(repo)
    .await?;

    let assignment_info_vals =
        assignment_info_views(repo, obj_dept, assignment_infos).await?;

    accept(assignment_info_vals)
}

/// Joins a chapter assignment with requested roles.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn join<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: JoinChapterAssignmentInstr,
) -> BaseRest<AssignmentInfoView>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let chapter_info = GetChapterInfo {
        id: &instr.chapter_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let member_info = MemberLoader::load_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &chapter_info.id,
    )
    .await?;

    chapter_perm_complex::ensure_user_can_join(&member_info, instr.roles)?;

    assignment_perm_complex::ensure_user_can_take_roles(
        &member_info,
        instr.roles,
    )?;

    let assignment_info = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            let existing_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &instr.chapter_id,
                user_id: &token.user_id,
            }
            .step_on(repo, context)
            .await?;

            let (assignment_info, workflow_record_payload) = if let Some(
                existing_assignment_info,
            ) =
                existing_assignment_info
            {
                //
                //
                let assignment_role_update = assignment_complex::merge_roles(
                    &existing_assignment_info,
                    instr.roles,
                );

                if assignment_role_update.roles
                    == existing_assignment_info.roles
                {
                    (existing_assignment_info, None)
                } else {
                    //
                    let assignment_info = UpdateAssignmentRoles {
                        update: &assignment_role_update,
                    }
                    .step_on(repo, context)
                    .await?;

                    let payload =
                        ChapterWorkflowRecordPayload::AssignmentRolesUpdated {
                            subject_user_id: assignment_info.user_id.clone(),
                            previous_roles: existing_assignment_info.roles,
                            next_roles: assignment_role_update.roles,
                        };

                    (assignment_info, Some(payload))
                }
            } else {
                //
                let assignment_entry = AssignmentEntry {
                    id: assignment_complex::gen_id(),
                    chapter_id: instr.chapter_id.into(),
                    user_id: token.user_id.as_str().into(),
                    roles: instr.roles,
                };

                let assignment_info = CreateAssignment {
                    entry: &assignment_entry,
                }
                .step_on(repo, context)
                .await?;

                let payload = ChapterWorkflowRecordPayload::AssignmentCreated {
                    subject_user_id: assignment_info.user_id.clone(),
                    roles: assignment_info.roles,
                };

                (assignment_info, Some(payload))
            };

            if let Some(payload) = workflow_record_payload {
                //
                let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                    chapter_info.id,
                    Some(token.user_id.into()),
                    payload,
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&workflow_record_entry),
                }
                .step_on(repo, context)
                .await?;
            }

            accept(assignment_info)
        })
        .await?;

    accept(AssignmentInfoView::from(assignment_info))
}

/// Deletes one assignment by identifier.
#[instrument(level = "info", skip(nucl, repo, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
{
    let assignment_info = GetAssignmentInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    if token.user_id != assignment_info.user_id {
        //
        let member_info = MemberLoader::load_info_from_chapter(
            repo,
            LoadMode::Run,
            &token.user_id,
            &assignment_info.chapter_id,
        )
        .await?;

        assignment_perm_complex::ensure_user_can_delete(
            &assignment_perm_complex::AssignmentDeleteAccess::Admin {
                member_info: &member_info,
            },
        )?;
    }

    if token.user_id == assignment_info.user_id {
        //
        assignment_perm_complex::ensure_user_can_delete(
            &assignment_perm_complex::AssignmentDeleteAccess::Owner,
        )?;
    }

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &assignment_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        chapter_complex::ensure_chapter_writable(&chapter_info)?;

        DeleteAssignments::Id { id: &id }
            .step_on(repo, context)
            .await?;

        let workflow_record_entry = ChapterWorkflowRecordEntry::new(
            chapter_info.id,
            Some(token.user_id.into()),
            ChapterWorkflowRecordPayload::AssignmentDeleted {
                subject_user_id: assignment_info.user_id,
                previous_roles: assignment_info.roles,
            },
        );

        CreateChapterWorkflowRecords {
            entries: std::slice::from_ref(&workflow_record_entry),
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

// Ensure the current user may list assignments for the requested scope.
async fn ensure_user_can_list<C, R>(
    repo: &R,
    token: &UserToken,
    assignment_list_spec: &AssignmentListSpec,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + MemberRepo<C> + TeamRepo<C> + UserRepo<C>,
{
    match assignment_list_spec {
        //
        AssignmentListSpec::Chapter { chapter_id, .. } => {
            //
            let team_id = ResolveTeamId::Chapter { id: chapter_id }
                .run_on(repo)
                .await?;

            let member_info = FindMemberInfo {
                user_id: &token.user_id,
                team_id: &team_id,
            }
            .run_on(repo)
            .await?;

            if let Some(member_info) = member_info {
                //
                return assignment_perm_complex::ensure_user_can_list_chapter_infos(
                    &assignment_perm_complex::AssignmentListAccess::Member {
                        member_info: &member_info,
                    },
                );
            }

            let assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id,
                user_id: &token.user_id,
            }
            .run_on(repo)
            .await?;

            let Some(assignment_info) = assignment_info else {
                //
                let err_message = trl("error-forbidden");

                tracing::warn!(
                    err_variant = ?ExpectedVariant::Perm,
                    err_message = %err_message,
                    user_id = %token.user_id,
                    chapter_id = %chapter_id,
                    team_id = %team_id,
                    "expected error: assignment list perm denied",
                );

                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Perm,
                    message: err_message,
                });
            };

            assignment_perm_complex::ensure_user_can_list_chapter_infos(
                &assignment_perm_complex::AssignmentListAccess::Assignee {
                    assignment_info: &assignment_info,
                },
            )
        }

        AssignmentListSpec::User { owner_id, .. }
            if token.user_id == *owner_id =>
        {
            assignment_perm_complex::ensure_user_can_list_user_infos(
                &assignment_perm_complex::UserAssignmentListAccess::Owner,
            )
        }

        AssignmentListSpec::User { owner_id, .. } => {
            //
            let user_info =
                GetUserInfo { id: &token.user_id }.run_on(repo).await?;

            assignment_perm_complex::ensure_user_can_list_user_infos(
                &assignment_perm_complex::UserAssignmentListAccess::SuperAdmin {
                    user_info: &user_info,
                },
            )
            .inspect_err(|_| {
                //
                tracing::warn!(
                    current_user_id = %token.user_id,
                    owner_id = %owner_id,
                    "assignment list permission denied",
                );
            })
        }
    }
}
