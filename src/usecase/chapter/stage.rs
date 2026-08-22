//! Chapter workflow-stage mutation use case.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::chapter::{ChapterComplex, ChapterPermComplex};
use crate::complex::image::ImageComplex;
use crate::data::instr::chapter::UpdateChapterStageInstr;
use crate::model::shared::user::UserToken;
use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::{Develop, EffectEvent as _};
use crate::part::nucl::ReptRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::DeferBatch;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
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
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
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
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<ReptRead>,
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
    let stage = Stage::from(instr.stage);

    let oper = StageOper::from(instr.oper);

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &instr.id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-chapter-workflow-role-required"),
        });
    };

    let assignment_infos = match oper {
        //
        StageOper::Advance => {
            //
            ListAssignmentInfos::Chapter {
                chapter_id: &instr.id,
                role: None,
                incls: &[],
            }
            .run_on(repo)
            .await?
        }

        StageOper::Revert => Vec::new(),
    };

    ChapterPermComplex::ensure_user_can_update_stage(
        &assignment_info,
        &assignment_infos,
        stage,
        oper,
    )?;

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

            let prev_phase = chapter_info.stages.get_phase(stage);

            let chapter_stage_update =
                ChapterComplex::build_stage_update(&chapter_info, stage, oper)?;

            let next_phase = chapter_stage_update.stages.get_phase(stage);

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
                        stage,
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

                if oper == StageOper::Advance
                    && prev_phase != StagePhase::Completed
                    && next_phase == StagePhase::Completed
                {
                    workflow_completed_chapter_id =
                        Some(chapter_info.id.clone());
                }

                if stage == Stage::Publish
                    && oper == StageOper::Advance
                    && !was_published
                    && next_phase == StagePhase::Completed
                {
                    clean_uploaded_images(
                        repo,
                        prom,
                        context,
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
        Event::ChapterWorkflowCompleted {
            payload: ChapterWorkflowCompletedEvent {
                chapter_id,
                completed_stage: stage,
            },
        }
        .develop_on(develop)
        .await;
    }

    if let Some(chapter_id) = published_chapter_id {
        //
        Event::ChapterPublished {
            payload: ChapterPublishedEvent { chapter_id },
        }
        .develop_on(develop)
        .await;
    }

    accept(())
}

// Clear uploaded page images and enqueue their object-storage deletions.
async fn clean_uploaded_images<C, R, P>(
    repo: &R,
    prom: &P,
    context: &mut C,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: PageRepo<C> + Sync,
    P: Prom<C> + Sync,
{
    let object_keys = ClearPageImagesForPublish { chapter_id }
        .step_on(repo, context)
        .await?;

    let delete_ids = object_keys
        .iter()
        .map(|_| ImageComplex::gen_delete_id())
        .collect::<Vec<_>>();

    let payloads = object_keys
        .into_iter()
        .map(|object_key| TaskPayload::Image {
            payload: image::ImagePayload::Delete { object_key },
        })
        .collect::<Vec<_>>();

    let tasks = delete_ids
        .iter()
        .zip(payloads.iter())
        .map(|(id, payload)| Task {
            id,
            payload,
            delay: None,
        })
        .collect::<Vec<_>>();

    DeferBatch::new(&tasks).step_on(prom, context).await?;

    accept(())
}
