//! Assignment use cases — list, join, role update, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::complex::chapter::ChapterPermComplex;
use crate::data::assignment_data;
use crate::model::assignment_model;
use crate::model::user_model;
use crate::part::image::ImagePool;
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{ExpectedVariant, RegularError, RegularResult};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists assignments by chapter or owner user.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: user_model::Token,
    data: assignment_data::ListInfosData,
) -> RegularResult<Vec<assignment_data::InfoVal>>
where
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + UserRepo<C>
        + Sync,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + UserRepoTransactional<C>,
    I: ImagePool,
{
    let assignment_list_spec: assignment_model::ListSpec = data.try_into()?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_list_infos(
        &mut repo.as_proxy(),
        &token.user_id,
        &assignment_list_spec,
    )
    .await?;

    let assignment_infos = repo
        .execute(&AssignmentStep::list_infos(&assignment_list_spec))
        .await?;

    let mut assignment_info_vals = Vec::with_capacity(assignment_infos.len());

    for assignment_info in assignment_infos {
        assignment_info_vals.push(
            assignment_data::InfoVal::from_model(image_pool, assignment_info)
                .await?,
        );
    }

    Ok(assignment_info_vals)
}

/// Joins a chapter assignment with requested roles.
pub async fn join<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    data: assignment_data::JoinChapterData,
) -> RegularResult<assignment_data::InfoVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + AssignmentRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    let chapter_info = repo
        .execute(&ChapterStep::get_info_by_id(&data.chapter_id, &[]))
        .await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_join(
        &mut repo.as_proxy(),
        &token.user_id,
        &chapter_info,
        data.roles,
    )
    .await?;

    AssignmentPermComplex::can_user_take_roles(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.chapter_id,
        data.roles,
    )
    .await?;

    let assignment_info = drive
        .with_context(
            async move |context| -> RegularResult<assignment_model::Info> {
                //
                let repo = repo.derive_transactional().await;

                let existing_assignment_info = repo
                    .advance(
                        context,
                        &AssignmentStep::get_info_by_chapter_id_and_user_id(
                            &data.chapter_id,
                            &token.user_id,
                        ),
                    )
                    .await?;

                let assignment_info = match existing_assignment_info {
                    //
                    Some(existing_assignment_info) => {
                        //
                        let assignment_role_update =
                            AssignmentComplex::merge_roles(
                                &existing_assignment_info,
                                data.roles,
                            );

                        repo.advance(
                            context,
                            &AssignmentStep::put_roles(&assignment_role_update),
                        )
                        .await?
                    }

                    None => {
                        //
                        let assignment_form = assignment_model::Form {
                            id: AssignmentComplex::gen_id(),
                            chapter_id: data.chapter_id,
                            user_id: token.user_id,
                            roles: data.roles,
                        };

                        repo.advance(
                            context,
                            &AssignmentStep::create(&assignment_form),
                        )
                        .await?
                    }
                };

                Ok(assignment_info)
            },
        )
        .await?;

    Ok(assignment_data::InfoVal::from(assignment_info))
}

/// Updates assignment roles.
pub async fn update_roles<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    data: assignment_data::UpdateRolesData,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_update_roles(
        &mut repo.as_proxy(),
        &token.user_id,
        &data,
    )
    .await?;

    AssignmentPermComplex::can_user_take_roles(
        &mut repo.as_proxy(),
        &data.user_id,
        &data.chapter_id,
        data.roles,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            let locked_assignment_infos = repo
                .advance(
                    context,
                    &AssignmentStep::list_infos_by_chapter_id_excluded(
                        &data.chapter_id,
                    ),
                )
                .await?;

            let existing_assignment_info =
                locked_assignment_infos.iter().find(|assignment_info| {
                    assignment_info.user_id == data.user_id
                });

            match existing_assignment_info {
                //
                Some(assignment_info) => {
                    //
                    if AssignmentComplex::is_self_admin_role_removal(
                        &token.user_id,
                        assignment_info,
                        data.roles,
                    ) {
                        return Err(assignment_admin_required_error());
                    }

                    if !AssignmentComplex::chapter_has_admin_after_role_update(
                        &locked_assignment_infos,
                        &data.user_id,
                        data.roles,
                    ) {
                        return Err(assignment_admin_required_error());
                    }

                    let assignment_role_update = assignment_model::RoleUpdate {
                        id: assignment_info.id.clone(),
                        roles: data.roles,
                    };

                    repo.advance(
                        context,
                        &AssignmentStep::put_roles(&assignment_role_update),
                    )
                    .await?;
                }

                None => {
                    //
                    if !AssignmentComplex::chapter_has_admin_after_role_update(
                        &locked_assignment_infos,
                        &data.user_id,
                        data.roles,
                    ) {
                        return Err(assignment_admin_required_error());
                    }

                    let assignment_form = assignment_model::Form {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: data.chapter_id,
                        user_id: data.user_id,
                        roles: data.roles,
                    };

                    repo.advance(
                        context,
                        &AssignmentStep::create(&assignment_form),
                    )
                    .await?;
                }
            }

            Ok(())
        })
        .await?;

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
pub async fn delete<D, C, R>(
    drive: &D,
    repo: &R,
    token: user_model::Token,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        AssignmentRepoTransactional<C> + Send + Sync,
{
    let assignment_info = repo
        .execute(&AssignmentStep::get_info_by_id(&id, &[]))
        .await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_delete(
        &mut repo.as_proxy(),
        &token.user_id,
        &assignment_info,
    )
    .await?;

    drive
        .with_context(async move |context| -> RegularResult<()> {
            //
            let repo = repo.derive_transactional().await;

            repo.advance(context, &AssignmentStep::delete(&id)).await?;

            Ok(())
        })
        .await?;

    Ok(())
}
