use serde::{Deserialize, Serialize};

use crate::result::RootResult;

#[derive(Serialize, Deserialize)]
pub enum StagePhase {
    Pending,
    Active,
    Completed,
}

#[derive(Serialize, Deserialize)]
#[serde(rename = "kebab-case")]
pub enum WorkflowStage {
    /// Named for "图源".
    RawProvide,
    /// Named for "翻译".
    Translate,
    /// Named for "校对".
    Proofread,
    /// Named for "嵌字/美工".
    TypesetRedraw,
    /// Named for "监修".
    Review,
    /// Named for "上传".
    Publish,
}

pub fn is_valid_stage_phase(stage: WorkflowStage, phase: StagePhase) -> bool {
    todo!()
}

#[derive(Serialize, Deserialize)]
pub enum WorkflowEvent {
    Advance,
    Revert,
}

pub fn try_modify_stage(
    curr: (WorkflowStage, StagePhase),
    event: WorkflowEvent,
) -> RootResult<StagePhase> {
    todo!()
}
