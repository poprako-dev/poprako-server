//! Chapter use cases — list, read, create, update, and deletion.

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::data::chapter::{
    ChapterInfoVal, CreateChapterData, CreateChapterVal, ListChapterInfosData,
    PatchChapterInfoData, UpdateChapterStageData,
};
use crate::model::assignment::AssignmentForm;
use crate::model::chapter::{ChapterForm, ChapterInfoUpdate, ChapterListSpec};
use crate::model::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload, ChapterWorkflowRevertedPayload,
};
use crate::part::effect::{EffectDevelop, EffectEmit as _};
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
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
use crate::result::{RegularError, RegularResult, accept};
use crate::util::DeriveTransactional;
use crate::value::chapter::{Stage, StageOper, StagePhase};
use crate::value::role::{RoleField, RoleMask};
use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

#[cfg(test)]
mod tests;

/// Lists chapters under one comic.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: ListChapterInfosData,
) -> RegularResult<Vec<ChapterInfoVal>>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
    I: ImagePool,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_list_infos(&mut repo.as_proxy(), &token.user_id, &data.comic_id)
        .await?;

    let spec = ChapterListSpec {
        comic_id: data.comic_id,
        incl_opt: data.incl_opt,
        offset: data.offset,
        limit: data.limit,
    };

    let chapter_infos = repo.execute(&ChapterStep::list_infos(&spec)).await?;

    let mut chapter_info_vals = Vec::with_capacity(chapter_infos.len());

    for chapter_info in chapter_infos {
        chapter_info_vals.push(ChapterInfoVal::from_model(image_pool, chapter_info).await?);
    }

    accept(chapter_info_vals)
}

/// Fetches a chapter by ID.
pub async fn get_info<C, R>(repo: &R, token: UserToken, id: String) -> RegularResult<ChapterInfoVal>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_get_info(&mut repo.as_proxy(), &token.user_id, &id).await?;

    let chapter_info = repo.execute(&ChapterStep::get_info_by_id(&id, &[])).await?;

    accept(ChapterInfoVal::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
pub async fn get_pinned<C, R>(
    repo: &R,
    token: UserToken,
    comic_id: String,
) -> RegularResult<Option<ChapterInfoVal>>
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
        .execute(&ChapterStep::find_pinned_info_by_comic_id(&comic_id, &[]))
        .await?;

    accept(chapter_info.map(ChapterInfoVal::from))
}

/// Creates a new chapter.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateChapterData,
) -> RegularResult<CreateChapterVal>
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
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_create(&mut repo.as_proxy(), &token.user_id, &data.comic_id)
        .await?;

    let chapter_id = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            repo.advance(
                context,
                &ChapterStep::list_all_infos_by_comic_id_excluded(&data.comic_id),
            )
            .await?;

            let index = repo
                .advance(context, &ComicStep::incr_chapter_next_index(&data.comic_id))
                .await?;

            let subtitle = ChapterComplex::subtitle_or_default(data.subtitle, index);
            let chapter_id = ChapterComplex::gen_id();

            repo.advance(
                context,
                &ChapterStep::unpin_others(&data.comic_id, &chapter_id),
            )
            .await?;

            let chapter_form = ChapterForm {
                id: chapter_id,
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
    data: PatchChapterInfoData,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + Send
        + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_update_info(&mut repo.as_proxy(), &token.user_id, &data.id)
        .await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let chapter_info = repo
                .advance(
                    context,
                    &ChapterStep::get_info_by_id_excluded(&data.id, &[]),
                )
                .await?;

            if data.subtitle.is_some() || data.pin.is_some() {
                let chapter_info_update = ChapterInfoUpdate {
                    id: data.id.clone(),
                    subtitle: data.subtitle,
                    pin: data.pin,
                };

                if chapter_info_update.pin == Some(true) {
                    repo.advance(
                        context,
                        &ChapterStep::list_all_infos_by_comic_id_excluded(&chapter_info.comic_id),
                    )
                    .await?;

                    repo.advance(
                        context,
                        &ChapterStep::unpin_others(&chapter_info.comic_id, &chapter_info.id),
                    )
                    .await?;
                }

                repo.advance(context, &ChapterStep::update_info(&chapter_info_update))
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

/// Updates chapter workflow state.
pub async fn update_stage<D, C, R, P, V>(
    drive: &D,
    repo: &R,
    prom: &P,
    develop: &V,
    token: UserToken,
    data: UpdateChapterStageData,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + PageRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + AssignmentRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    V: EffectDevelop + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_update_stage(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.id,
        data.stage,
        data.oper,
    )
    .await?;

    let events = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let chapter_info = repo
                .advance(
                    context,
                    &ChapterStep::get_info_by_id_excluded(&data.id, &[]),
                )
                .await?;

            let was_published =
                chapter_info.stages.get_phase(Stage::Publish) == StagePhase::Completed;

            let previous_phase = chapter_info.stages.get_phase(data.stage);

            let chapter_stage_update =
                ChapterComplex::build_stage_update(&chapter_info, data.stage, data.oper)?;

            let next_phase = chapter_stage_update.stages.get_phase(data.stage);

            repo.advance(context, &ChapterStep::update_stage(&chapter_stage_update))
                .await?;

            let mut events = Vec::new();

            if data.oper == StageOper::Advance
                && previous_phase != StagePhase::Completed
                && next_phase == StagePhase::Completed
            {
                events.push(Event::ChapterWorkflowCompleted(
                    ChapterWorkflowCompletedPayload {
                        chapter_id: chapter_info.id.clone(),
                        completed_stage: data.stage,
                    },
                ));
            }

            if data.stage == Stage::Publish
                && data.oper == StageOper::Advance
                && !was_published
                && chapter_stage_update
                    .stages
                    .has_phase(Stage::Publish, StagePhase::Completed)
            {
                ChapterComplex::clean_uploaded_images(&repo, prom, context, &chapter_info.id)
                    .await?;

                events.push(Event::ChapterPublished(ChapterPublishedPayload {
                    chapter_id: chapter_info.id.clone(),
                }));
            }

            if data.oper == StageOper::Revert && previous_phase != next_phase {
                events.push(Event::ChapterWorkflowReverted(
                    ChapterWorkflowRevertedPayload {
                        chapter_id: chapter_info.id.clone(),
                        reverted_stage: data.stage,
                    },
                ));
            }

            repo.advance(
                context,
                &ComicStep::touch_last_active(&chapter_info.comic_id),
            )
            .await?;

            accept(events)
        })
        .await
        .map_err(map_drive_err)?;

    events.emit(develop).await;

    accept(())
}

/// Deletes one chapter and its descendant core records.
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
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + PageRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: ChapterRepoTransactional<C>
        + ComicRepoTransactional<C>
        + WorksetRepoTransactional<C>
        + MemberRepoTransactional<C>
        + PageRepoTransactional<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    ChapterPermComplex::can_user_delete(&mut repo.as_proxy(), &token.user_id, &id).await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            ChapterComplex::delete_cascade(&repo, prom, context, &id).await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}
