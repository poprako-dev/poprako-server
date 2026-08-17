//! Chapter workflow-stage mutation use case.

use poprako_orchestra::{
    AtLeast, Context, Nucl, OperStep as _, run_proxy, step_proxy,
};
use tracing::instrument;

use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::data::instr::chapter::UpdateChapterStageInstr;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::nucl::RepeatableRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::DeferBatch;
use crate::part::prom::payload::TaskPayload;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::oper::assignment::{
    FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, UpdateChapterStage,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page::ClearPageImagesForPublish;
use crate::part::repo::page::PageRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::value::chapter::{Stage, StageOper, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Updates one chapter workflow stage and records the real phase transition.
#[instrument(level = "info", skip(nucl, repo, prom, develop))]
pub async fn update_stage<N, C, R, P, D>(
    (nucl, repo, prom, develop): (&N, &R, &P, &D),
    token: UserToken,
    instr: UpdateChapterStageInstr,
) -> BaseRest<()>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + PageRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
    D: Develop + Send + Sync,
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

    let (workflow_completed_chapter_id, published_chapter_id) = nucl
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

            let next_phase = chapter_stage_update.stages.get_phase(instr.stage);

            let mut workflow_completed_chapter_id = None;

            let mut published_chapter_id = None;

            if prev_phase != next_phase {
                //
                UpdateChapterStage {
                    update: &chapter_stage_update,
                }
                .step_on(repo, context)
                .await?;

                let workflow_record_entry = ChapterWorkflowRecordEntry::new(
                    chapter_info.id.clone(),
                    Some(token.user_id.clone()),
                    ChapterWorkflowRecordPayload::StageTransitioned {
                        stage: instr.stage,
                        previous_phase: prev_phase,
                        next_phase,
                        origin: ChapterWorkflowRecordOrigin::Manual,
                    },
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&workflow_record_entry),
                }
                .step_on(repo, context)
                .await?;

                if instr.oper == StageOper::Advance
                    && prev_phase != StagePhase::Completed
                    && next_phase == StagePhase::Completed
                {
                    workflow_completed_chapter_id =
                        Some(chapter_info.id.clone());
                }

                if instr.stage == Stage::Publish
                    && instr.oper == StageOper::Advance
                    && !was_published
                    && next_phase == StagePhase::Completed
                {
                    let guarded_repo =
                        &crate::part::nucl::GuardedStep::new(repo);

                    let guarded_prom =
                        &crate::part::nucl::GuardedStep::new(prom);

                    ChapterComplex::clean_uploaded_images(
                        &mut step_proxy! {
                            context;
                            guarded_repo => for<'a> ClearPageImagesForPublish<'a>;
                            guarded_prom => for<'t, 'a> DeferBatch<'t, 'a, String, TaskPayload, ()>;
                        },
                        &chapter_info.id,
                    )
                    .await?;

                    published_chapter_id = Some(chapter_info.id.clone());
                }
            }

            TouchComicLastActive {
                id: &chapter_info.comic_id,
            }
            .step_on(repo, context)
            .await?;

            accept((workflow_completed_chapter_id, published_chapter_id))
        })
        .await?;

    if let Some(chapter_id) = workflow_completed_chapter_id {
        //
        Event::ChapterWorkflowCompleted(ChapterWorkflowCompletedEvent {
            chapter_id,
            completed_stage: instr.stage,
        })
        .develop_on(develop)
        .await;
    }

    if let Some(chapter_id) = published_chapter_id {
        //
        Event::ChapterPublished(ChapterPublishedEvent { chapter_id })
            .develop_on(develop)
            .await;
    }

    accept(())
}
