//! Assignment use cases — list, join, role update, and deletion.

use poprako_orchestra::{Nucl, run_proxy};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::complex::chapter::ChapterPermComplex;
use crate::complex::comic::ComicComplex;
use crate::data::assignment::{
    AssignmentInfoVal, JoinChapterAssignmentParams, ListAssignmentInfosParams,
    UpdateAssignmentRolesParams,
};
use crate::model::assignment::{
    AssignmentEntry, AssignmentInfoListSpec, AssignmentRoleUpdate,
};
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, DeleteAssignments, FindAssignmentInfo, GetAssignmentInfo,
    ListAssignmentInfos, ListAssignmentInfosExcluded, UpdateAssignmentRoles,
};
use crate::part::repo::oper::chapter::{
    GetChapterInfo, ListPinnedChapterInfos,
};
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::ListFirstPageInfos;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseResult, ExpectedVariant, accept};

#[cfg(test)]
mod tests;

/// Lists assignments by chapter or owner user.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListAssignmentInfosParams,
) -> BaseResult<Vec<AssignmentInfoVal>>
where
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + UserRepo<C>
        + PageRepo<C>
        + Sync,
    I: ImagePool,
{
    let assignment_list_spec: AssignmentInfoListSpec = params.try_into()?;

    AssignmentPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>,
                for<'a> GetUserInfo<'a>;
        },
        &token.user_id,
        &assignment_list_spec,
    )
    .await?;

    let assignment_infos = repo
        .run(&ListAssignmentInfos::Spec {
            spec: &assignment_list_spec,
        })
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
            AssignmentInfoVal::from_model(
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn join<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: JoinChapterAssignmentParams,
) -> BaseResult<AssignmentInfoVal>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
{
    let chapter_info = repo
        .run(&GetChapterInfo {
            id: &params.chapter_id,
            incls: &[],
        })
        .await?;

    ChapterPermComplex::ensure_user_can_join(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> GetChapterInfo<'a, 'b>;
        },
        &token.user_id,
        &chapter_info,
        params.roles,
    )
    .await?;

    AssignmentPermComplex::ensure_user_can_take_roles(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> GetChapterInfo<'a, 'b>;
        },
        &token.user_id,
        &params.chapter_id,
        params.roles,
    )
    .await?;

    let assignment_info = nucl
        .coord(async move |context| {
            //

            let existing_assignment_info = repo
                .step(
                    context,
                    &FindAssignmentInfo::ChapterUser {
                        chapter_id: &params.chapter_id,
                        user_id: &token.user_id,
                    },
                )
                .await?;

            match existing_assignment_info {
                //
                Some(existing_assignment_info) => {
                    //
                    let assignment_role_update = AssignmentComplex::merge_roles(
                        &existing_assignment_info,
                        params.roles,
                    );

                    repo.step(
                        context,
                        &UpdateAssignmentRoles {
                            update: &assignment_role_update,
                        },
                    )
                    .await
                }

                None => {
                    //
                    let assignment_entry = AssignmentEntry {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: params.chapter_id,
                        user_id: token.user_id,
                        roles: params.roles,
                    };

                    repo.step(
                        context,
                        &CreateAssignment {
                            entry: &assignment_entry,
                        },
                    )
                    .await
                }
            }
        })
        .await?;

    accept(AssignmentInfoVal::from(assignment_info))
}

/// Updates assignment roles.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn update_roles<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateAssignmentRolesParams,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    AssignmentPermComplex::ensure_user_can_update_roles(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params,
    )
    .await?;

    AssignmentPermComplex::ensure_user_can_take_roles(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>,
                for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &params.user_id,
        &params.chapter_id,
        params.roles,
    )
    .await?;

    nucl.coord(async move |context| {
        //

        let assignment_infos = repo
            .step(
                context,
                &ListAssignmentInfosExcluded::Chapter {
                    chapter_id: &params.chapter_id,
                },
            )
            .await?;

        let existing_assignment_info = assignment_infos
            .iter()
            .find(|assignment_info| assignment_info.user_id == params.user_id);

        match existing_assignment_info {
            //
            Some(assignment_info) => {
                //
                if AssignmentComplex::is_self_admin_role_removal(
                    &token.user_id,
                    assignment_info,
                    params.roles,
                ) {
                    return Err(assignment_admin_required_error());
                }

                if !AssignmentComplex::chapter_has_admin_after_role_update(
                    &assignment_infos,
                    &params.user_id,
                    params.roles,
                ) {
                    return Err(assignment_admin_required_error());
                }

                let assignment_role_update = AssignmentRoleUpdate {
                    id: assignment_info.id.clone(),
                    roles: params.roles,
                };

                repo.step(
                    context,
                    &UpdateAssignmentRoles {
                        update: &assignment_role_update,
                    },
                )
                .await?;
            }

            None => {
                //
                if !AssignmentComplex::chapter_has_admin_after_role_update(
                    &assignment_infos,
                    &params.user_id,
                    params.roles,
                ) {
                    return Err(assignment_admin_required_error());
                }

                let assignment_entry = AssignmentEntry {
                    id: AssignmentComplex::gen_id(),
                    chapter_id: params.chapter_id,
                    user_id: params.user_id,
                    roles: params.roles,
                };

                repo.step(
                    context,
                    &CreateAssignment {
                        entry: &assignment_entry,
                    },
                )
                .await?;
            }
        }

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}

/// Constructs a permission error for admin-role removal.
fn assignment_admin_required_error() -> BaseError {
    BaseError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-forbidden"),
    }
}

/// Deletes one assignment by identifier.
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn delete<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
) -> BaseResult<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: AssignmentRepo<C> + Send + Sync,
{
    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &id,
            incls: &[],
        })
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

        repo.step(context, &DeleteAssignments::Id { id: &id })
            .await?;

        accept(())
    })
    .await?;

    let () = ();

    accept(())
}
