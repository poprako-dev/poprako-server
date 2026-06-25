//! Workset use cases — create, read, update, list, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::workset::WorksetComplex;
use crate::data::workset::{
    WorksetCreateData, WorksetCreateVal, WorksetInfoUpdateData, WorksetInfoVal, WorksetListData,
};
use crate::model::workset::{WorksetForm, WorksetInfoUpdate};
use crate::part::repo::map_drive_err;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub mod tests;

/// Creates a new workset inside a team.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    data: WorksetCreateData,
) -> RootResult<WorksetCreateVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: TeamRepo<C> + WorksetRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        TeamRepoTransactional<C> + WorksetRepoTransactional<C> + Send,
{
    let workset_info = drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;
            let index = repo
                .advance(
                    context,
                    &TeamStep::increment_workset_next_index(&data.team_id),
                )
                .await?;
            let workset_form = WorksetForm {
                id: WorksetComplex::gen_id(),
                team_id: data.team_id,
                index,
                name: data.name,
                description: data.description,
            };
            let workset_info = repo
                .advance(context, &WorksetStep::create(&workset_form))
                .await?;
            accept(workset_info)
        })
        .await
        .map_err(map_drive_err)?;

    Ok(WorksetCreateVal {
        workset: workset_info.into(),
    })
}

/// Fetches a workset by ID.
pub async fn get_info<C, R>(repo: &R, id: String) -> RootResult<WorksetInfoVal>
where
    R: WorksetRepo<C>,
    <R as DeriveTransactional>::Transactional: WorksetRepoTransactional<C>,
{
    let workset_info = repo.execute(&WorksetStep::get_info_by_id(&id)).await?;

    Ok(workset_info.into())
}

/// Lists worksets for a team.
pub async fn list_infos<C, R>(repo: &R, data: WorksetListData) -> RootResult<Vec<WorksetInfoVal>>
where
    R: WorksetRepo<C>,
    <R as DeriveTransactional>::Transactional: WorksetRepoTransactional<C>,
{
    let infos = repo
        .execute(&WorksetStep::list_by_team_id(&data.team_id))
        .await?;

    Ok(infos.into_iter().map(Into::into).collect())
}

/// Updates a workset's name and description.
pub async fn update_info<C, R>(repo: &R, data: WorksetInfoUpdateData) -> RootResult<()>
where
    R: WorksetRepo<C>,
    <R as DeriveTransactional>::Transactional: WorksetRepoTransactional<C>,
{
    let update = WorksetInfoUpdate {
        id: data.id,
        name: data.name,
        description: data.description,
    };

    repo.execute(&WorksetStep::update_info(&update)).await?;

    Ok(())
}

/// Deletes a workset and its child data.
pub async fn delete<D, C, R>(drive: &D, repo: &R, id: String) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: WorksetRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: WorksetRepoTransactional<C> + Send,
{
    drive
        .with_context(async move |context| {
            let repo = DeriveTransactional::transactional(repo).await;

            // FIXME: workset complex::delete cascade
            repo.advance(context, &WorksetStep::delete_cascade(&id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)
}
