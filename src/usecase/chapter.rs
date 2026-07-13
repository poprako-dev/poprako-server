//! Chapter use cases — list, read, create, update, and deletion.

use poprako_orchestra::{Nucl, run_proxy};

use crate::complex::assignment::AssignmentComplex;
use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::data::chapter::{
    ChapterInfoVal, CreateChapterParams, CreateChapterPayload,
    ListChapterInfosParams, UpdateChapterInfoParams, UpdateChapterStageParams,
};
use crate::model::assignment::AssignmentEntry;
use crate::model::chapter::{
    ChapterEntry, ChapterInfoListSpec, ChapterInfoUpdate,
};
use crate::model::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedPayload, ChapterWorkflowCompletedPayload,
};
use crate::part::effect::{EffectDevelop, EffectEmit as _};
use crate::part::image::ImagePool;
use crate::part::prom::Prom;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    CreateAssignment, FindAssignmentInfo,
};
use crate::part::repo::oper::chapter::{
    CreateChapter, FindPinnedChapterInfo, GetChapterInfo,
    GetChapterInfoExcluded, ListChapterInfos, ListChapterInfosExcluded,
    UnpinOtherChapters, UpdateChapter, UpdateChapterStage,
};
use crate::part::repo::oper::comic::{
    AllocateComicChapterIndex, GetComicInfo, TouchComicLastActive,
    UpdateComicChapterCount,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult, accept};
use crate::value::chapter::{Stage, StageOper, StagePhase};
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
mod tests;

/// Lists chapters under one comic.
pub async fn list_infos<C, R, I>(
    repo: &R,
    image_pool: &I,
    token: UserToken,
    params: ListChapterInfosParams,
) -> RegularResult<Vec<ChapterInfoVal>>
where
    R: ChapterRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
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
        &params.comic_id,
    )
    .await?;

    let spec = ChapterInfoListSpec {
        comic_id: params.comic_id,
        incl_opt: params.incl_opt,
        offset: params.offset,
        limit: params.limit,
    };

    let chapter_infos = repo.run(&ListChapterInfos { spec: &spec }).await?;

    let mut chapter_info_vals = Vec::with_capacity(chapter_infos.len());

    for chapter_info in chapter_infos {
        chapter_info_vals
            .push(ChapterInfoVal::from_model(image_pool, chapter_info).await?);
    }

    Ok(chapter_info_vals)
}

/// Fetches a chapter by ID.
pub async fn get_info<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<ChapterInfoVal>
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

    let chapter_info = repo
        .run(&GetChapterInfo {
            id: &id,
            incls: &[],
        })
        .await?;

    Ok(ChapterInfoVal::from(chapter_info))
}

/// Fetches the pinned chapter under one comic.
pub async fn get_pinned<C, R>(
    repo: &R,
    token: UserToken,
    comic_id: String,
) -> RegularResult<Option<ChapterInfoVal>>
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

    let chapter_info = repo
        .run(&FindPinnedChapterInfo {
            comic_id: &comic_id,
            incls: &[],
        })
        .await?;

    Ok(chapter_info.map(ChapterInfoVal::from))
}

/// Creates a new chapter.
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateChapterParams,
) -> RegularResult<CreateChapterPayload>
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
    ChapterPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.comic_id,
    )
    .await?;

    let chapter_id = nucl
        .coord(async move |context| -> RegularResult<String> {
            //
            repo.step(
                context,
                &ListChapterInfosExcluded {
                    comic_id: &params.comic_id,
                },
            )
            .await?;

            let index = repo
                .step(
                    context,
                    &AllocateComicChapterIndex {
                        id: &params.comic_id,
                    },
                )
                .await?;

            let subtitle =
                ChapterComplex::subtitle_or_default(params.subtitle, index);

            let chapter_id = ChapterComplex::gen_id();

            repo.step(
                context,
                &UnpinOtherChapters {
                    comic_id: &params.comic_id,
                    excluded_id: &chapter_id,
                },
            )
            .await?;

            let chapter_entry = ChapterEntry {
                id: chapter_id,
                comic_id: params.comic_id,
                is_pinned: true,
                index,
                subtitle,
                creator_id: token.user_id.clone(),
            };

            let chapter_info = repo
                .step(
                    context,
                    &CreateChapter {
                        entry: &chapter_entry,
                    },
                )
                .await?;

            repo.step(
                context,
                &UpdateComicChapterCount {
                    id: &chapter_info.comic_id,
                    delta: 1,
                },
            )
            .await?;

            repo.step(
                context,
                &TouchComicLastActive {
                    id: &chapter_info.comic_id,
                },
            )
            .await?;

            let assignment_entry = AssignmentEntry {
                id: AssignmentComplex::gen_id(),
                chapter_id: chapter_info.id.clone(),
                user_id: token.user_id,
                roles: RoleMask::from(RoleField::ADMIN),
            };

            repo.step(
                context,
                &CreateAssignment {
                    entry: &assignment_entry,
                },
            )
            .await?;

            Ok(chapter_info.id)
        })
        .await?;

    Ok(CreateChapterPayload { id: chapter_id })
}

/// Updates chapter metadata.
pub async fn update_info<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: UpdateChapterInfoParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: ChapterRepo<C> + ComicRepo<C> + AssignmentRepo<C> + Send + Sync,
{
    ChapterPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params.id,
    )
    .await?;

    nucl.coord(async move |context| -> RegularResult<()> {
        //
        let chapter_info = repo
            .step(
                context,
                &GetChapterInfoExcluded {
                    id: &params.id,
                    incls: &[],
                },
            )
            .await?;

        if params.subtitle.is_some() || params.pin.is_some() {
            //
            let chapter_info_update = ChapterInfoUpdate {
                id: params.id.clone(),
                subtitle: params.subtitle,
                pin: params.pin,
            };

            if chapter_info_update.pin == Some(true) {
                //

                repo.step(
                    context,
                    &ListChapterInfosExcluded {
                        comic_id: &chapter_info.comic_id,
                    },
                )
                .await?;

                repo.step(
                    context,
                    &UnpinOtherChapters {
                        comic_id: &chapter_info.comic_id,
                        excluded_id: &chapter_info.id,
                    },
                )
                .await?;
            }

            repo.step(
                context,
                &UpdateChapter {
                    update: &chapter_info_update,
                },
            )
            .await?;
        }

        repo.step(
            context,
            &TouchComicLastActive {
                id: &chapter_info.comic_id,
            },
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Updates chapter workflow state.
pub async fn update_stage<N, C, R, P, V>(
    nucl: &N,
    repo: &R,
    prom: &P,
    develop: &V,
    token: UserToken,
    params: UpdateChapterStageParams,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
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
            repo => for<'a, 'b> FindAssignmentInfo<'a, 'b>;
        },
        &token.user_id,
        &params.id,
        params.stage,
        params.oper,
    )
    .await?;

    let events = nucl
        .coord(async move |context| -> RegularResult<Vec<Event>> {
            //
            let chapter_info = repo
                .step(
                    context,
                    &GetChapterInfoExcluded {
                        id: &params.id,
                        incls: &[],
                    },
                )
                .await?;

            let was_published = chapter_info.stages.get_phase(Stage::Publish)
                == StagePhase::Completed;

            let previous_phase = chapter_info.stages.get_phase(params.stage);

            let chapter_stage_update = ChapterComplex::build_stage_update(
                &chapter_info,
                params.stage,
                params.oper,
            )?;

            let next_phase =
                chapter_stage_update.stages.get_phase(params.stage);

            repo.step(
                context,
                &UpdateChapterStage {
                    update: &chapter_stage_update,
                },
            )
            .await?;

            let mut events = Vec::new();

            if params.oper == StageOper::Advance
                && previous_phase != StagePhase::Completed
                && next_phase == StagePhase::Completed
            {
                events.push(Event::ChapterWorkflowCompleted(
                    ChapterWorkflowCompletedPayload {
                        chapter_id: chapter_info.id.clone(),
                        completed_stage: params.stage,
                    },
                ));
            }

            if params.stage == Stage::Publish
                && params.oper == StageOper::Advance
                && !was_published
                && chapter_stage_update
                    .stages
                    .has_phase(Stage::Publish, StagePhase::Completed)
            {
                // TODO: archive this chapter and relevant assignments.

                ChapterComplex::clean_uploaded_images(
                    repo,
                    prom,
                    context,
                    &chapter_info.id,
                )
                .await?;

                events.push(Event::ChapterPublished(ChapterPublishedPayload {
                    chapter_id: chapter_info.id.clone(),
                }));
            }

            repo.step(
                context,
                &TouchComicLastActive {
                    id: &chapter_info.comic_id,
                },
            )
            .await?;

            Ok(events)
        })
        .await?;

    events.emit(develop).await;

    accept(())
}

/// Deletes one chapter and its descendant core records.
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    ChapterPermComplex::ensure_user_can_delete(
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

    nucl.coord(async move |context| -> RegularResult<()> {
        //
        ChapterComplex::delete_cascade(repo, prom, context, &id).await?;

        accept(())
    })
    .await?;

    accept(())
}
