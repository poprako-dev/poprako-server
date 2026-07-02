//! Workset use cases — create, read, update, list, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::page::Page;
use poprako_util::time::ToUnixMilli;

use crate::complex::workset::{WorksetComplex, WorksetPermComplex};
use crate::data::workset::{
    CreateWorksetData, CreateWorksetVal, ListWorksetInfosData, UpdateWorksetInfoData,
    WorksetInfoVal,
};
use crate::model::user::UserToken;
use crate::model::workset::{WorksetForm, WorksetInfoUpdate};
use crate::part::prom::Prom;
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::team::{TeamRepo, TeamRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RegularError, RegularResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub mod tests;

/// Creates a new workset inside a team.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateWorksetData,
) -> RegularResult<CreateWorksetVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: TeamRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: TeamRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    WorksetPermComplex::can_user_create(&mut repo.as_proxy(), &token.user_id, &data.team_id)
        .await?;

    let workset_id = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

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

            accept(workset_info.id)
        })
        .await
        .map_err(map_drive_err)?;

    Ok(CreateWorksetVal { id: workset_id })
}

/// Fetches a workset by ID.
pub async fn get_info<C, R>(repo: &R, token: UserToken, id: String) -> RegularResult<WorksetInfoVal>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    WorksetPermComplex::can_user_get_info(&mut repo.as_proxy(), &token.user_id, &id).await?;

    let workset_info = repo.execute(&WorksetStep::get_info_by_id(&id)).await?;

    Ok(WorksetInfoVal {
        id: workset_info.id,
        team_id: workset_info.team_id,
        index: workset_info.index,
        name: workset_info.name,
        description: workset_info.description,
        comic_count: workset_info.comic_count,
        comic_next_index: workset_info.comic_next_index,
        created_at: workset_info.created_at.to_unix_milli(),
        updated_at: workset_info.updated_at.to_unix_milli(),
    })
}

/// Lists worksets for a team.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListWorksetInfosData,
) -> RegularResult<Vec<WorksetInfoVal>>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    WorksetPermComplex::can_user_list_infos(&mut repo.as_proxy(), &token.user_id, &data.team_id)
        .await?;

    let workset_infos = repo
        .execute(&WorksetStep::list_infos_by_team_id(
            &data.team_id,
            Page {
                offset: data.offset,
                limit: data.limit,
            },
        ))
        .await?;

    let workset_info_vals = workset_infos
        .into_iter()
        .map(|workset_info| WorksetInfoVal {
            id: workset_info.id,
            team_id: workset_info.team_id,
            index: workset_info.index,
            name: workset_info.name,
            description: workset_info.description,
            comic_count: workset_info.comic_count,
            comic_next_index: workset_info.comic_next_index,
            created_at: workset_info.created_at.to_unix_milli(),
            updated_at: workset_info.updated_at.to_unix_milli(),
        })
        .collect();

    Ok(workset_info_vals)
}

/// Updates a workset's name and description.
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    data: UpdateWorksetInfoData,
) -> RegularResult<()>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        WorksetRepoTransactional<C> + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    WorksetPermComplex::can_user_update_info(&mut repo.as_proxy(), &token.user_id, &data.id)
        .await?;

    let workset_info_update = WorksetInfoUpdate {
        id: data.id,
        name: data.name,
        description: data.description,
    };

    repo.execute(&WorksetStep::update_info(&workset_info_update))
        .await?;

    accept(())
}

/// Deletes a workset and its child data.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: WorksetRepo<C> + ComicRepo<C> + MemberRepo<C> + ChapterRepo<C> + PageRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: WorksetRepoTransactional<C>
        + ComicRepoTransactional<C>
        + MemberRepoTransactional<C>
        + ChapterRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    WorksetPermComplex::can_user_delete(&mut repo.as_proxy(), &token.user_id, &id).await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let workset_info = repo
                .advance(context, &WorksetStep::get_info_excluded(&id))
                .await?;

            WorksetComplex::delete_cascade(&repo, prom, context, &workset_info.id).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
