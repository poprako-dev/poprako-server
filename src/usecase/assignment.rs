//! Assignment use cases — list, join, role update, and deletion.

#[cfg(test)]
// Unit tests that cover assignment orchestration invariants.
mod tests;

use poprako_orchestra::{
    AtLeast, Nucl, OperRun as _, OperStep as _, run_proxy,
};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::complex::comic::ComicComplex;
use crate::data::instr::assignment::{
    JoinChapterAssignmentInstr, ListAssignmentInfosInstr,
    UpdateAssignmentRolesInstr,
};
use crate::data::view::assignment::AssignmentInfoView;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::{AssignmentEntry, AssignmentRoleRepl};
use crate::part::image::ImagePool;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part::repo::oper::chapter::{
    GetChapterInfo, GetChapterInfoExcluded, ListPinnedChapterInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Lists assignments by chapter or owner user.
#[instrument(level = "info", skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListAssignmentInfosInstr,
) -> BaseRest<Vec<AssignmentInfoView>>
where
    C: poprako_orchestra::Context,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + UserRepo<C>
        + PageRepo<C>
        + Sync,
    I: ImagePool,
{
    let assignment_list_spec = instr.try_into()?;

    AssignmentPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>,
                for<'a> GetUserInfo<'a>;
        },
        &token.user_id,
        &assignment_list_spec,
    )
    .await?;

    let assignment_infos = ListAssignmentInfos::Spec {
        spec: &assignment_list_spec,
    }
    .run_on(repo)
    .await?;

    let comic_ids = assignment_infos
        .iter()
        .filter_map(|assignment_info| assignment_info.chapter.as_ref())
        .filter_map(|chapter_info| chapter_info.comic.as_ref())
        .map(|comic_info| comic_info.id.clone())
        .collect::<Vec<_>>();

    let fallback_cover_keys = ComicComplex::resolve_fallback_cover_keys(
        &mut run_proxy! {
            repo =>
                for<'a> ListPinnedChapterInfos<'a>,
                for<'a> ListFirstPageInfos<'a>;
        },
        &comic_ids,
    )
    .await?;

    let mut assignment_info_vals = Vec::with_capacity(assignment_infos.len());

    for assignment_info in assignment_infos {
        //
        let fallback_cover_key = assignment_info
            .chapter
            .as_ref()
            .and_then(|chapter_info| chapter_info.comic.as_ref())
            .and_then(|comic_info| fallback_cover_keys.get(&comic_info.id))
            .map(String::as_str);

        assignment_info_vals.push(
            AssignmentInfoView::from_model(
                image_pool,
                assignment_info,
                fallback_cover_key,
            )
            .await?,
        );
    }

    accept(assignment_info_vals)
}

/// Joins a chapter assignment with requested roles.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn join<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: JoinChapterAssignmentInstr,
) -> BaseRest<AssignmentInfoView>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C>
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

    ChapterPermComplex::ensure_user_can_join(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &chapter_info,
        instr.roles,
    )
    .await?;

    AssignmentPermComplex::ensure_user_can_take_roles(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.chapter_id,
        instr.roles,
    )
    .await?;

    let assignment_info = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let existing_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &instr.chapter_id,
                user_id: &token.user_id,
            }
            .step_on(repo, context)
            .await?;

            match existing_assignment_info {
                //
                Some(existing_assignment_info) => {
                    //
                    let assignment_role_update = AssignmentComplex::merge_roles(
                        &existing_assignment_info,
                        instr.roles,
                    );

                    UpdateAssignmentRoles {
                        update: &assignment_role_update,
                    }
                    .step_on(repo, context)
                    .await
                }

                None => {
                    //
                    let assignment_entry = AssignmentEntry {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: instr.chapter_id,
                        user_id: token.user_id,
                        roles: instr.roles,
                    };

                    CreateAssignment {
                        entry: &assignment_entry,
                    }
                    .step_on(repo, context)
                    .await
                }
            }
        })
        .await?;

    accept(AssignmentInfoView::from(assignment_info))
}

/// Updates assignment roles.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn update_roles<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateAssignmentRolesInstr,
) -> BaseRest<()>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + Send
        + Sync,
{
    AssignmentPermComplex::ensure_user_can_update_roles(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &instr.user_id,
        &instr.chapter_id,
        instr.roles,
    )
    .await?;

    AssignmentPermComplex::ensure_user_can_take_roles(
        &mut run_proxy! {
            repo =>
                for<'a> ResolveTeamId<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &instr.user_id,
        &instr.chapter_id,
        instr.roles,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &instr.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        let assignment_infos = ListAssignmentInfosExcluded::Chapter {
            chapter_id: &instr.chapter_id,
        }
        .step_on(repo, context)
        .await?;

        let existing_assignment_info = assignment_infos
            .iter()
            .find(|assignment_info| assignment_info.user_id == instr.user_id);

        match existing_assignment_info {
            //
            Some(assignment_info) => {
                //
                if AssignmentComplex::is_self_admin_role_removal(
                    &token.user_id,
                    assignment_info,
                    instr.roles,
                ) {
                    //
                    let err_message = trl("error-forbidden");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Perm,
                        err_message = %err_message,
                        chapter_id = %instr.chapter_id,
                        user_id = %token.user_id,
                        affected_user_id = %instr.user_id,
                        roles = ?instr.roles,
                        operation = "remove own administrator role",
                        "expected error: chapter administrator perm required",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Perm,
                        message: err_message,
                    });
                }

                if !AssignmentComplex::chapter_has_admin_after_role_update(
                    &assignment_infos,
                    &instr.user_id,
                    instr.roles,
                ) {
                    //
                    let err_message = trl("error-forbidden");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Perm,
                        err_message = %err_message,
                        chapter_id = %instr.chapter_id,
                        user_id = %token.user_id,
                        affected_user_id = %instr.user_id,
                        roles = ?instr.roles,
                        operation = "remove last chapter administrator role",
                        "expected error: chapter administrator perm required",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Perm,
                        message: err_message,
                    });
                }

                let assignment_role_update = AssignmentRoleRepl {
                    id: assignment_info.id.clone(),
                    roles: instr.roles,
                };

                UpdateAssignmentRoles {
                    update: &assignment_role_update,
                }
                .step_on(repo, context)
                .await?;
            }

            None => {
                //
                if !AssignmentComplex::chapter_has_admin_after_role_update(
                    &assignment_infos,
                    &instr.user_id,
                    instr.roles,
                ) {
                    //
                    let err_message = trl("error-forbidden");

                    tracing::warn!(
                        err_variant = ?ExpectedVariant::Perm,
                        err_message = %err_message,
                        chapter_id = %instr.chapter_id,
                        user_id = %token.user_id,
                        affected_user_id = %instr.user_id,
                        roles = ?instr.roles,
                        operation = "assign administrator role",
                        "expected error: chapter administrator perm required",
                    );

                    return Err(BaseError::Expected {
                        variant: ExpectedVariant::Perm,
                        message: err_message,
                    });
                }

                let assignment_entry = AssignmentEntry {
                    id: AssignmentComplex::gen_id(),
                    chapter_id: instr.chapter_id,
                    user_id: instr.user_id,
                    roles: instr.roles,
                };

                CreateAssignment {
                    entry: &assignment_entry,
                }
                .step_on(repo, context)
                .await?;
            }
        }

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

/// Deletes one assignment by identifier.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: poprako_orchestra::Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: AssignmentRepo<C> + ChapterRepo<C> + Send + Sync,
{
    let assignment_info = GetAssignmentInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    AssignmentPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &assignment_info,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &assignment_info.chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        DeleteAssignments::Id { id: &id }
            .step_on(repo, context)
            .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}
