//! Chapter use cases — list, read, create, update, and deletion.

use poprako_orchestra::{
    Nucl, OperRun as _, OperStep as _, run_proxy, step_proxy,
};
use poprako_orchestra_extra::prom::oper::DeferBatch;
use tracing::instrument;

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::complex::comic::ComicComplex;
use crate::data::instr::chapter::{
    CreateChapterInstr, ListChapterInfosInstr, UpdateChapterInfoInstr,
    UpdateChapterStageInstr,
};
use crate::data::val::chapter::CreateChapterVal;
use crate::data::view::chapter::ChapterInfoView;
use crate::model::read::spec::chapter::ChapterListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::assignment::AssignmentEntry;
use crate::model::write::chapter::{ChapterEntry, ChapterPatch};
use crate::part::effect::EffectDevelop;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::chapter::{
    CreateChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListPinnedChapterInfos,
    LockChapters, UnpinOtherChapters, UpdateChapter, UpdateChapterStage,
};
use crate::part::repo::oper::comic::{
    AllocComicChapterIndex, GetComicInfo, TouchComicLastActive,
    UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::page::{
    ClearPageImagesForPublish, ListFirstPageInfos,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::{Stage, StageOper, StagePhase};

pub use delete::delete;

// Chapter deletion use cases (internal).
mod delete;

#[cfg(test)]
mod tests;

/// Lists chapters under one comic.
#[instrument(level = "info", err(Debug), skip(repo, image_pool))]
pub async fn list_infos<C, R, I>(
    (repo, image_pool): (&R, &I),
    token: UserToken,
    instr: ListChapterInfosInstr,
) -> BaseRest<Vec<ChapterInfoView>>
where
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + PageRepo<C>
        + Sync,
    I: ImagePool,
{
    ChapterPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.comic_id,
    )
    .await?;

    let spec = ChapterListSpec {
        comic_id: instr.comic_id,
        incl_opt: instr.incl_opt,
        offset: instr.offset,
        limit: instr.limit,
    };

    let chapter_infos = ListChapterInfos { spec: &spec }.run_on(repo).await?;

    let comic_ids = chapter_infos
        .iter()
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

    let mut chapter_info_vals = Vec::with_capacity(chapter_infos.len());

    for chapter_info in chapter_infos {
        //
        let fallback_cover_key = chapter_info
            .comic
            .as_ref()
            .and_then(|comic_info| fallback_cover_keys.get(&comic_info.id))
            .map(String::as_str);

        chapter_info_vals.push(
            ChapterInfoView::from_model(
                image_pool,
                chapter_info,
                fallback_cover_key,
            )
            .await?,
        );
    }

    accept(chapter_info_vals)
}

/// Fetches a chapter by ID.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<ChapterInfoView>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    ChapterPermComplex::ensure_user_can_get_info(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetChapterInfo<'a, 'b>,
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let chapter_info = GetChapterInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(ChapterInfoView::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn get_pinned<C, R>(
    (repo,): (&R,),
    token: UserToken,
    comic_id: String,
) -> BaseRest<Option<ChapterInfoView>>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    ChapterPermComplex::ensure_user_can_get_pinned(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &comic_id,
    )
    .await?;

    let chapter_info = FindPinnedChapterInfo {
        comic_id: &comic_id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    accept(chapter_info.map(ChapterInfoView::from))
}

/// Creates a new chapter.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateChapterInstr,
) -> BaseRest<CreateChapterVal>
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
    ChapterPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &instr.comic_id,
        instr.preset_assignment_roles,
    )
    .await?;

    let chapter_id = nucl
        .coord(async move |context| {
            //
            LockChapters {
                comic_id: &instr.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let index = AllocComicChapterIndex {
                id: &instr.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let subtitle =
                ChapterComplex::subtitle_or_default(instr.subtitle, index);

            let chapter_id = ChapterComplex::gen_id();

            UnpinOtherChapters {
                comic_id: &instr.comic_id,
                excluded_id: &chapter_id,
            }
            .step_on(repo, context)
            .await?;

            let chapter_entry = ChapterEntry {
                id: chapter_id,
                comic_id: instr.comic_id,
                is_pinned: true,
                index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = CreateChapter {
                entry: &chapter_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateComicChapterCount {
                id: &chapter_info.comic_id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            let assignment_entry = AssignmentEntry {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id,
                roles: AssignmentComplex::creator_roles(
                    instr.preset_assignment_roles,
                ),
            };

            CreateAssignment {
                entry: &assignment_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(chapter_info.id)
        })
        .await?;

    accept(CreateChapterVal { id: chapter_id })
}

/// Updates chapter metadata.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateChapterInfoInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    ChapterPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &instr.id,
    )
    .await?;

    nucl.coord(async move |context| {
        //
        let chapter_info = GetChapterInfoExcluded {
            id: &instr.id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ChapterComplex::ensure_chapter_writable(&chapter_info)?;

        if instr.subtitle.is_some() {
            //
            let chapter_info_update = ChapterPatch {
                id: instr.id.clone(),
                subtitle: instr.subtitle,
                pin: None,
            };

            UpdateChapter {
                update: &chapter_info_update,
            }
            .step_on(repo, context)
            .await?;
        }

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Marks a chapter as the pinned chapter for its comic.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn mark_pinned<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    ChapterPermComplex::ensure_user_can_mark_pinned(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let chapter_info = GetChapterInfo {
        id: &id,
        incls: &[],
    }
    .run_on(repo)
    .await?;

    let comic_id = chapter_info.comic_id;

    let () = nucl
        .coord(async move |context| {
            //
            LockChapters {
                comic_id: &comic_id,
            }
            .step_on(repo, context)
            .await?;

            let chapter_info = GetChapterInfoExcluded {
                id: &id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            UnpinOtherChapters {
                comic_id: &chapter_info.comic_id,
                excluded_id: &chapter_info.id,
            }
            .step_on(repo, context)
            .await?;

            let chapter_info_update = ChapterPatch {
                id: chapter_info.id.clone(),
                subtitle: None,
                pin: Some(true),
            };

            UpdateChapter {
                update: &chapter_info_update,
            }
            .step_on(repo, context)
            .await?;

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(())
        })
        .await?;

    accept(())
}

/// Updates chapter workflow state.
#[instrument(level = "info", err(Debug), skip(nucl, repo, prom, develop))]
pub async fn update_stage<N, C, R, P, V>(
    (nucl, repo, prom, develop): (&N, &R, &P, &V),
    token: UserToken,
    instr: UpdateChapterStageInstr,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    V: EffectDevelop + Send + Sync,
{
    ChapterPermComplex::ensure_user_can_update_stage(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> FindAssignmentInfo<'a, 'b>,
                for<'a, 'b> ListAssignmentInfos<'a, 'b>;
        },
        &token.user_id,
        &instr.id,
        instr.stage,
        instr.oper,
    )
    .await?;

    let events = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            ChapterComplex::ensure_chapter_writable(&chapter_info)?;

            let was_published = chapter_info.stages.get_phase(Stage::Publish)
                == StagePhase::Completed;

            let prev_phase = chapter_info.stages.get_phase(instr.stage);

            let chapter_stage_update = ChapterComplex::build_stage_update(
                &chapter_info,
                instr.stage,
                instr.oper,
            )?;

            let next_phase =
                chapter_stage_update.stages.get_phase(instr.stage);

            UpdateChapterStage {
                update: &chapter_stage_update,
            }
            .step_on(repo, context)
            .await?;

            let mut events = Vec::new();

            if instr.oper == StageOper::Advance
                && prev_phase != StagePhase::Completed
                && next_phase == StagePhase::Completed
            {
                events.push(Event::ChapterWorkflowCompleted(
                    ChapterWorkflowCompletedPayload {
                        chapter_id: chapter_info.id.clone(),
                        completed_stage: instr.stage,
                    },
                ));
            }

            if instr.stage == Stage::Publish
                && instr.oper == StageOper::Advance
                && !was_published
                && chapter_stage_update
                    .stages
                    .has_phase(Stage::Publish, StagePhase::Completed)
            {
                // TODO: archive this chapter and relevant assignments.

                ChapterComplex::clean_uploaded_images(
                    &mut step_proxy! {
                        context;
                        repo => for<'a> ClearPageImagesForPublish<'a>;
                        prom => for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>;
                    },
                    &chapter_info.id,
                )
                .await?;

                events.push(Event::ChapterPublished(ChapterPublishedPayload {
                    chapter_id: chapter_info.id.clone(),
                }));
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept(events)
        })
        .await?;

    develop.develop(events).await;

    accept(())
}
