//! Chapter use cases — list, read, create, update, join, and deletion.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::data::chapter::{
    AssignmentInfoVal, ChapterInfoVal, CreateChapterData, CreateChapterVal, JoinChapterData,
    ListChapterInfosData, UpdateChapterInfoData, UpdateChapterStageData,
};
use crate::model::assignment::AssignmentForm;
use crate::model::chapter::{ChapterForm, ChapterInfoUpdate};
use crate::model::role::{RoleField, RoleMask};
use crate::model::user::UserToken;
use crate::part::prom::{Prom, PromTransactional};
use crate::part::repo::assignment::{AssignmentRepo, AssignmentRepoTransactional};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::page::{PageRepo, PageRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{RootError, RootResult, accept};
use crate::util::DeriveTransactional;
use crate::value::chapter::{StagePhase, WorkflowEvent, WorkflowStage};

#[cfg(test)]
mod tests;

/// Lists chapters under one comic.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListChapterInfosData,
) -> RootResult<Vec<ChapterInfoVal>>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_list_infos(&mut repo.as_proxy(), &token.user_id, &data.comic_id)
        .await?;

    let chapter_infos = repo
        .execute(&ChapterStep::list_infos_by_comic_id(
            &data.comic_id,
            data.offset,
            data.limit,
        ))
        .await?;

    accept(
        chapter_infos
            .into_iter()
            .map(ChapterInfoVal::from)
            .collect(),
    )
}

/// Fetches a chapter by ID.
pub async fn get_info<C, R>(repo: &R, token: UserToken, id: String) -> RootResult<ChapterInfoVal>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_get_info(&mut repo.as_proxy(), &token.user_id, &id).await?;

    let chapter_info = repo.execute(&ChapterStep::get_info_by_id(&id)).await?;

    accept(ChapterInfoVal::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
pub async fn get_pinned<C, R>(
    repo: &R,
    token: UserToken,
    comic_id: String,
) -> RootResult<Option<ChapterInfoVal>>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_get_pinned(&mut repo.as_proxy(), &token.user_id, &comic_id)
        .await?;

    let chapter_info = repo
        .execute(&ChapterStep::find_pinned_info_by_comic_id(&comic_id))
        .await?;

    accept(chapter_info.map(ChapterInfoVal::from))
}

/// Creates a new chapter.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateChapterData,
) -> RootResult<CreateChapterVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
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
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        ChapterPermComplex::can_user_create(&mut repo.as_proxy(), &token.user_id, &data.comic_id)
            .await?;
    }

    let chapter_id = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let index = repo
                .advance(context, &ComicStep::incr_chapter_next_index(&data.comic_id))
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(data.subtitle, index);

            let chapter_form = ChapterForm {
                id: ChapterComplex::gen_id(),
                comic_id: data.comic_id,
                is_pinned: true,
                index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = repo
                .advance(context, &ChapterStep::create(&chapter_form))
                .await?;

            repo.advance(
                context,
                &ChapterStep::unpin_others(&chapter_info.comic_id, &chapter_info.id),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::update_chapter_count(&chapter_info.comic_id, 1),
            )
            .await?;

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            let assignment_form = AssignmentForm {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id,
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.advance(context, &AssignmentStep::create(&assignment_form))
                .await?;

            accept(chapter_info.id)
        })
        .await
        .map_err(map_drive_err)?;

    Ok(CreateChapterVal { id: chapter_id })
}

/// Updates chapter metadata.
pub async fn update_info<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: UpdateChapterInfoData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        ChapterPermComplex::can_user_update_info(&mut repo.as_proxy(), &token.user_id, &data.id)
            .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

            let chapter_info = repo
                .advance(context, &ChapterStep::get_info_by_id_excluded(&data.id))
                .await?;

            if data.subtitle.is_some() || data.pin.is_some() {
                let chapter_info_update = ChapterInfoUpdate {
                    id: data.id.clone(),
                    subtitle: data.subtitle,
                    pin: data.pin,
                };

                repo.advance(context, &ChapterStep::update_info(&chapter_info_update))
                    .await?;

                if chapter_info_update.pin == Some(true) {
                    repo.advance(
                        context,
                        &ChapterStep::unpin_others(&chapter_info.comic_id, &chapter_info.id),
                    )
                    .await?;
                }
            }

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Updates chapter workflow state.
pub async fn update_stage<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    data: UpdateChapterStageData,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + PageRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        ChapterPermComplex::can_user_update_stage(
            &mut repo.as_proxy(),
            &token.user_id,
            &data.id,
            data.stage,
            data.event,
        )
        .await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            let chapter_info = repo
                .advance(context, &ChapterStep::get_info_by_id_excluded(&data.id))
                .await?;

            let was_published =
                chapter_info.stages.get_phase(WorkflowStage::Publish) == StagePhase::Completed;

            let chapter_stage_update =
                ChapterComplex::build_stage_update(&chapter_info, data.stage, data.event)?;

            repo.advance(context, &ChapterStep::update_stage(&chapter_stage_update))
                .await?;

            if data.stage == WorkflowStage::Publish
                && data.event == WorkflowEvent::Advance
                && !was_published
                && chapter_stage_update
                    .stages
                    .has_phase(WorkflowStage::Publish, StagePhase::Completed)
            {
                ChapterComplex::clean_uploaded_images(&repo, &prom, context, &chapter_info.id)
                    .await?;
            }

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Joins a chapter assignment with requested roles.
pub async fn join<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: JoinChapterData,
) -> RootResult<AssignmentInfoVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
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
        .execute(&ChapterStep::get_info_by_id(&data.chapter_id))
        .await?;

    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        ChapterPermComplex::can_user_join(
            &mut repo.as_proxy(),
            &token.user_id,
            &chapter_info,
            data.roles,
        )
        .await?;
    }

    let assignment_info = drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;

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
                Some(existing_assignment_info) => {
                    let assignment_role_update =
                        AssignmentComplex::merge_roles(&existing_assignment_info, data.roles);

                    repo.advance(context, &AssignmentStep::put_roles(&assignment_role_update))
                        .await?
                }
                None => {
                    let assignment_form = AssignmentForm {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: data.chapter_id,
                        user_id: token.user_id,
                        roles: data.roles,
                    };

                    repo.advance(context, &AssignmentStep::create(&assignment_form))
                        .await?
                }
            };

            accept(assignment_info)
        })
        .await
        .map_err(map_drive_err)?;

    accept(AssignmentInfoVal::from(assignment_info))
}

/// Deletes one chapter and its descendant core records.
pub async fn delete<D, C, R, P>(
    drive: &D,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RootResult<()>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + PageRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    <P as DeriveTransactional>::Transactional: PromTransactional<C> + Send + Sync,
{
    {
        use crate::part::shared::proxy::AsProxyNonTransactional as _;

        ChapterPermComplex::can_user_delete(&mut repo.as_proxy(), &token.user_id, &id).await?;
    }

    drive
        .with_context(async move |context| {
            let repo = repo.transactional().await;
            let prom = prom.transactional().await;

            ChapterComplex::delete_cascade(&repo, &prom, context, &id).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
