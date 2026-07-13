//! Assignment use cases — list, join, role update, and deletion.

use poprako_orchestra::{Nucl, run_proxy};

use poprako_util::i18n::trl;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::complex::chapter::ChapterPermComplex;
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment::JoinChapterAssignmentParams;
use crate::data::assignment::ListAssignmentInfosParams;
use crate::data::assignment::UpdateAssignmentRolesParams;
use crate::model::assignment::AssignmentEntry;
use crate::model::assignment::AssignmentInfo;
use crate::model::assignment::AssignmentInfoListSpec;
use crate::model::assignment::AssignmentRoleUpdate;
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
use crate::part::repo::oper::chapter::GetChapterInfo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::user::UserRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{ExpectedVariant, RegularError, RegularResult};

#[cfg(test)]
mod tests;

/// Lists assignments by chapter or owner user.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListAssignmentInfosParams,
) -> RegularResult<Vec<AssignmentInfoVal>>
where
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + UserRepo<C>
        + Sync,
    I: ImagePool,
{
    let assignment_list_spec: AssignmentInfoListSpec = params.try_into()?;

    AssignmentPermComplex::can_user_list_infos(
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

    let list_assignment_infos = ListAssignmentInfos::Spec {
        spec: &assignment_list_spec,
    };

    let assignment_infos = repo.run(&list_assignment_infos).await?;

    let mut assignment_info_vals = Vec::with_capacity(assignment_infos.len());

    for assignment_info in assignment_infos {
        assignment_info_vals.push(
            AssignmentInfoVal::from_model(image_pool, assignment_info).await?,
        );
    }

    Ok(assignment_info_vals)
}

/// Joins a chapter assignment with requested roles.
pub async fn join<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: JoinChapterAssignmentParams,
) -> RegularResult<AssignmentInfoVal>
where
    N: Nucl<Context = C, Error = RegularError>,
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

    ChapterPermComplex::can_user_join(
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

    AssignmentPermComplex::can_user_take_roles(
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
        .coord(async move |context| -> RegularResult<AssignmentInfo> {
            let find_assignment_info = FindAssignmentInfo::ChapterUser {
                chapter_id: &params.chapter_id,
                user_id: &token.user_id,
            };

            let existing_assignment_info =
                repo.step(context, &find_assignment_info).await?;

            match existing_assignment_info {
                Some(existing_assignment_info) => {
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

    Ok(AssignmentInfoVal::from(assignment_info))
}

/// Updates assignment roles.
pub async fn update_roles<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateAssignmentRolesParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    AssignmentPermComplex::can_user_update_roles(
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

    AssignmentPermComplex::can_user_take_roles(
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

    let transaction_output = nucl
        .coord(async move |context| -> RegularResult<()> {
            let list_assignment_infos_excluded =
                ListAssignmentInfosExcluded::Chapter {
                    chapter_id: &params.chapter_id,
                };

            let assignment_infos =
                repo.step(context, &list_assignment_infos_excluded).await?;

            let existing_assignment_info =
                assignment_infos.iter().find(|assignment_info| {
                    assignment_info.user_id == params.user_id
                });

            match existing_assignment_info {
                Some(assignment_info) => {
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

            Ok(())
        })
        .await?;

    let () = transaction_output;

    Ok(())
}

/// Constructs a permission error for admin-role removal.
fn assignment_admin_required_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-forbidden"),
    }
}

/// Deletes one assignment by identifier.
pub async fn delete<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: AssignmentRepo<C> + Send + Sync,
{
    let assignment_info = repo
        .run(&GetAssignmentInfo {
            id: &id,
            incls: &[],
        })
        .await?;

    AssignmentPermComplex::can_user_delete(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &assignment_info,
    )
    .await?;

    let transaction_output = nucl
        .coord(async move |context| -> RegularResult<()> {
            let delete_assignment = DeleteAssignments::Id { id: &id };

            repo.step(context, &delete_assignment).await?;

            Ok(())
        })
        .await?;

    let () = transaction_output;

    Ok(())
}
