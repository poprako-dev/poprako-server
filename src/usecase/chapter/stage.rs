//! Chapter workflow-stage mutation use case.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::key::ObjKind as _;
use poprako_obj_dept::oper::{ClearObjs, ListObjMetas};
use poprako_obj_dept::{ObjDept, ObjDeptView};
use poprako_util::i18n::trl;

use crate::complex::chapter as chapter_complex;
use crate::complex::chapter::perm as chapter_perm_complex;
use crate::data::instr::chapter::UpdateChapterStageInstr;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::shared::user::UserToken;
use crate::part::effect::event::Event;
use crate::part::effect::event::chapter::{
    ChapterPublishedEvent, ChapterWorkflowCompletedEvent,
};
use crate::part::effect::{EffectEvent as _, EventBus};
use crate::part::nucl::ReptRead;
use crate::part::obj_dept::{ChapterArtwork, PageSourceImage};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::assignment::{
    FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::chapter::{
    CompleteChapterRawProvide, GetChapterInfoExcluded, UpdateChapterStage,
};
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::page_source::{
    ListPageSourceInfos, UpdatePageSourceRawIdents,
};
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::team::TeamRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};
use crate::shape::write::chapter_workflow_record::ChapterWorkflowRecordEntryShape;
use crate::shape::write::page_source::PageSourceRawIdentsReplShape;
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;
use crate::value::chapter::stage::{Stage, StageOper, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};
use crate::value::role::RoleField;

/// Outcome of checking whether raw provisioning can advance.
pub enum RawProvideAdvance {
    /// One or more page objects are not available yet.
    Pending,

    /// The chapter already required no transition.
    Unchanged,

    /// Raw provisioning advanced and emitted its completion event.
    Advanced,
}

/// Advances raw provisioning after every current page object is available.
#[instrument(level = "info", skip(nucl, repo, obj_dept_view, bus))]
pub async fn try_advance_raw_provide<C, N, R, V, B>(
    (nucl, repo, obj_dept_view, bus): (&N, &R, &V, &B),
    chapter_id: &str,
    actor_user_id: Option<String>,
) -> BaseRest<RawProvideAdvance>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageSourceRepo<C>
        + Send
        + Sync,
    V: ObjDeptView<PageSourceImage, C> + Sync,
    B: EventBus + Sync,
{
    let is_advanced = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            if !chapter_info
                .stages
                .has_phase(Stage::RawProvide, StagePhase::Pending)
                || chapter_info.page_source_count == 0
            {
                return accept!(Some(false));
            }

            let page_source_infos = ListPageSourceInfos { chapter_id }
                .step_on(repo, context)
                .await?;

            let page_source_ids = page_source_infos
                .iter()
                .map(|page_source_info| page_source_info.id.as_str())
                .collect::<Vec<_>>();

            let page_source_image_ids = PageSourceImage::ids(&page_source_ids)?;

            let obj_metas =
                ListObjMetas::<PageSourceImage>::new(&page_source_image_ids)
                    .step_on(obj_dept_view, context)
                    .await
                    .map_err(BaseError::from)?;

            let are_images_uploaded =
                page_source_infos.iter().all(|page_source_info| {
                    //
                    obj_metas
                        .get(page_source_info.id.as_str())
                        .is_some_and(|obj_meta| obj_meta.is_avail)
                });

            if !are_images_uploaded {
                return accept!(None);
            }

            let is_advanced = CompleteChapterRawProvide { id: chapter_id }
                .step_on(repo, context)
                .await?;

            if is_advanced {
                //
                let entry = ChapterWorkflowRecordEntryShape::new(
                    chapter_id,
                    actor_user_id.as_deref(),
                    ChapterWorkflowRecordPayload::StageTransitioned {
                        stage: Stage::RawProvide,
                        previous_phase: StagePhase::Pending,
                        next_phase: StagePhase::Completed,
                        origin: ChapterWorkflowRecordOrigin::RawProvideCheck,
                    },
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&entry),
                }
                .step_on(repo, context)
                .await?;
            }

            accept!(Some(is_advanced))
        })
        .await?;

    match is_advanced {
        //
        Some(true) => {
            //
            Event::ChapterWorkflowCompleted {
                payload: ChapterWorkflowCompletedEvent {
                    chapter_id: chapter_id.to_string(),
                    completed_stage: Stage::RawProvide,
                },
            }
            .emit_on(bus)
            .await;

            accept!(RawProvideAdvance::Advanced)
        }

        Some(false) => accept!(RawProvideAdvance::Unchanged),

        None => accept!(RawProvideAdvance::Pending),
    }
}

/// Updates one chapter workflow stage and records the real phase transition.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, bus, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
pub async fn update_stage<N, C, R, O, B>(
    (nucl, repo, obj_dept, bus): (&N, &R, &O, &B),
    token: UserToken,
    instr: UpdateChapterStageInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<ReptRead>,
    R: ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + ComicRepo<C>
        + AssignmentRepo<C>
        + MemberRepo<C>
        + TeamRepo<C>
        + PageSourceRepo<C>
        + Send
        + Sync,
    O: ObjDept<ChapterArtwork, C> + ObjDept<PageSourceImage, C> + Send + Sync,
    B: EventBus + Send + Sync,
{
    let stage = Stage::from(instr.stage);

    let oper = StageOper::from(instr.oper);

    ensure_update_stage_perm(repo, &token, &instr, stage, oper).await?;

    let (workflow_completed_chapter_id, published_chapter_id) = nucl
        .coord(async move |context| {
            //
            let chapter_info = GetChapterInfoExcluded {
                id: &instr.id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            chapter_complex::ensure_chapter_writable(&chapter_info)?;

            let was_published = chapter_info.stages.get_phase(Stage::Publish)
                == StagePhase::Completed;

            let prev_phase = chapter_info.stages.get_phase(stage);

            let chapter_stage_update = chapter_complex::build_stage_update(
                &chapter_info,
                stage,
                oper,
            )?;

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

                let entry = ChapterWorkflowRecordEntryShape::new(
                    chapter_info.id.as_str(),
                    Some(token.user_id.as_str()),
                    ChapterWorkflowRecordPayload::StageTransitioned {
                        stage,
                        previous_phase: prev_phase,
                        next_phase,
                        origin: ChapterWorkflowRecordOrigin::Manual,
                    },
                );

                CreateChapterWorkflowRecords {
                    entries: std::slice::from_ref(&entry),
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
                    clear_chapter_files(
                        repo,
                        obj_dept,
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

            accept!((workflow_completed_chapter_id, published_chapter_id))
        })
        .await?;

    emit_stage_events(
        bus,
        stage,
        workflow_completed_chapter_id,
        published_chapter_id,
    )
    .await;

    accept!(())
}

// Validates the caller's assignment and workflow-stage permission.
async fn ensure_update_stage_perm<C, R>(
    repo: &R,
    token: &UserToken,
    instr: &UpdateChapterStageInstr,
    stage: Stage,
    oper: StageOper,
) -> BaseRest<()>
where
    C: Context,
    R: AssignmentRepo<C> + MemberRepo<C> + TeamRepo<C> + Sync,
{
    let member_info = MemberLoader::find_info_from_chapter(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &instr.id,
    )
    .await?;

    if let Some(member_info) = member_info
        && member_info.roles.has_any_role(&[RoleField::ADMIN])
    {
        return chapter_perm_complex::ensure_user_can_manage(&member_info);
    }

    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id: &instr.id,
        user_id: &token.user_id,
    }
    .run_on(repo)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-chapter-workflow-role-required");
            "chapter workflow role required",
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

        StageOper::Revert => Vec::<AssignmentInfo>::new(),
    };

    chapter_perm_complex::ensure_user_can_update_stage(
        &assignment_info,
        &assignment_infos,
        stage,
        oper,
    )
}

// Clear chapter artwork and page images and defer their object-storage deletions.
async fn clear_chapter_files<C, R, O>(
    repo: &R,
    obj_dept: &O,
    context: &mut C,
    chapter_id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: PageSourceRepo<C> + Sync,
    O: ObjDept<ChapterArtwork, C> + ObjDept<PageSourceImage, C> + Sync,
{
    let chapter_artwork_ids = ChapterArtwork::ids(&[chapter_id.to_owned()])?;

    ClearObjs::<ChapterArtwork>::new(&chapter_artwork_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let page_source_infos = ListPageSourceInfos { chapter_id }
        .step_on(repo, context)
        .await?;

    let page_source_ids = page_source_infos
        .into_iter()
        .map(|page_source_info| page_source_info.id)
        .collect::<Vec<_>>();

    let page_source_image_ids = PageSourceImage::ids(&page_source_ids)?;

    ClearObjs::<PageSourceImage>::new(&page_source_image_ids)
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let raw_idents = page_source_ids
        .iter()
        .map(|page_source_id| (page_source_id.as_str(), None))
        .collect::<Vec<_>>();

    let raw_ident_repl = PageSourceRawIdentsReplShape { idents: raw_idents };

    UpdatePageSourceRawIdents {
        repl: &raw_ident_repl,
    }
    .step_on(repo, context)
    .await?;

    accept!(())
}

// Emits workflow completion and publication events after commit.
async fn emit_stage_events<B>(
    bus: &B,
    stage: Stage,
    workflow_completed_chapter_id: Option<String>,
    published_chapter_id: Option<String>,
) where
    B: EventBus + Sync,
{
    if let Some(chapter_id) = workflow_completed_chapter_id {
        //
        Event::ChapterWorkflowCompleted {
            payload: ChapterWorkflowCompletedEvent {
                chapter_id,
                completed_stage: stage,
            },
        }
        .emit_on(bus)
        .await;
    }

    if let Some(chapter_id) = published_chapter_id {
        //
        Event::ChapterPublished {
            payload: ChapterPublishedEvent { chapter_id },
        }
        .emit_on(bus)
        .await;
    }
}
