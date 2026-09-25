//! Transactional automatic chapter-stage advancement helpers.

use poprako_orchestra::{Context, OperStep as _};

use crate::model::write::chapter_workflow_record::ChapterWorkflowRecordEntry;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::oper::chapter::StartChapterStage;
use crate::part::repo::oper::chapter_workflow_record::CreateChapterWorkflowRecords;
use crate::result::{BaseRest, accept};
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::chapter_workflow_record::{
    ChapterWorkflowRecordOrigin, ChapterWorkflowRecordPayload,
};

/// Identifies the chapter and actor for automatic stage transitions.
pub struct PendingStageStart<'a> {
    /// Chapter whose pending stages can start.
    chapter_id: &'a str,
    /// User responsible for the triggering action, when present.
    actor_user_id: Option<&'a str>,
    /// Business action that triggered the transitions.
    origin: ChapterWorkflowRecordOrigin,
}

impl<'a> PendingStageStart<'a> {
    /// Identifies one automatic stage transition trigger.
    pub const fn new(
        chapter_id: &'a str,
        actor_user_id: Option<&'a str>,
        origin: ChapterWorkflowRecordOrigin,
    ) -> Self {
        //
        Self {
            chapter_id,
            actor_user_id,
            origin,
        }
    }
}

/// Starts each still-pending stage and records every real transition atomically.
pub async fn start_pending_stages<C, R>(
    repo: &R,
    context: &mut C,
    start: PendingStageStart<'_>,
    stages: &[Stage],
) -> BaseRest<()>
where
    C: Context,
    R: ChapterRepo<C> + ChapterWorkflowRecordRepo<C> + Sync,
{
    let PendingStageStart {
        chapter_id,
        actor_user_id,
        origin,
    } = start;

    let mut entries = Vec::with_capacity(stages.len());

    for stage in stages {
        //
        let started = StartChapterStage {
            id: chapter_id,
            stage: *stage,
        }
        .step_on(repo, context)
        .await?;

        if started {
            //
            let payload = ChapterWorkflowRecordPayload::StageTransitioned {
                stage: *stage,
                previous_phase: StagePhase::Pending,
                next_phase: StagePhase::Active,
                origin,
            };

            entries.push(ChapterWorkflowRecordEntry::new(
                chapter_id,
                actor_user_id.map(Into::into),
                payload,
            ));
        }
    }

    CreateChapterWorkflowRecords { entries: &entries }
        .step_on(repo, context)
        .await?;

    accept(())
}
