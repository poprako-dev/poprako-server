//! Assignment use cases — listing and role mutation.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::data::assignment::{
    AssignmentInfoVal, ListAssignmentInfosData, UpdateAssignmentRoleData,
};
use crate::model::assignment::{AssignmentForm, AssignmentListSpec, AssignmentRoleUpdate};
use crate::model::user::UserToken;
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
mod tests;

/// Lists assignments by chapter or owner user.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListAssignmentInfosData,
) -> RootResult<Vec<AssignmentInfoVal>>
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
{
    let assignment_list_spec: AssignmentListSpec = data.try_into()?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        AssignmentPermComplex::can_user_list_infos(
            &mut repo.as_proxy(),
            &token.user_id,
            &assignment_list_spec,
        )
        .await?;
    }

    let assignment_infos = repo
        .execute(&AssignmentStep::list_infos(&assignment_list_spec))
        .await?;

    accept(
        assignment_infos
            .into_iter()
            .map(AssignmentInfoVal::from)
            .collect(),
    )
}

/// Updates assignment roles.
pub async fn update_roles<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: UpdateAssignmentRoleData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
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
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        AssignmentPermComplex::can_user_update_roles(&mut repo.as_proxy(), &token.user_id, &data)
            .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let existing_assignment_info = repo
                .advance(
                    context,
                    &AssignmentStep::get_info_by_chapter_id_and_user_id(
                        &data.chapter_id,
                        &data.user_id,
                    ),
                )
                .await?;

            match existing_assignment_info {
                Some(assignment_info) => {
                    let assignment_role_update = AssignmentRoleUpdate {
                        id: assignment_info.id,
                        roles: data.roles,
                    };

                    repo.advance(context, &AssignmentStep::put_roles(&assignment_role_update))
                        .await?;
                }
                None => {
                    let assignment_form = AssignmentForm {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: data.chapter_id,
                        user_id: data.user_id,
                        roles: data.roles,
                    };

                    repo.advance(context, &AssignmentStep::create(&assignment_form))
                        .await?;
                }
            }

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Deletes one assignment by identifier.
pub async fn delete<D, C, R>(drive: &D, repo: &R, token: UserToken, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: AssignmentRepoTransactional<C> + Send + Sync,
{
    let assignment_info = repo.execute(&AssignmentStep::get_info_by_id(&id)).await?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        AssignmentPermComplex::can_user_delete(
            &mut repo.as_proxy(),
            &token.user_id,
            &assignment_info,
        )
        .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            repo.advance(context, &AssignmentStep::delete(&id)).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
