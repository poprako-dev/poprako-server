//! Domain models for chapters inside comics — workflow state, progress counters,
//! and display metadata tracked per chapter.
//!
//! See [`StagePhase`] and [`WorkflowStage`](crate::value::chapter::WorkflowStage)
//! for the six production stages and their phase transitions.
//! Convert to [`ChapterInfoView`] for presentation.
//!
//! [`ChapterInfoView`]: crate::data::view::chapter::ChapterInfoView
//! [`StagePhase`]: crate::value::chapter::stage::StagePhase

use crate::value::chapter::mask::StageMask;

/// Mutable non-workflow fields for a chapter.
///
/// The profile update endpoint changes only `subtitle`. Chapter pinning is a
/// separate action, while internal orchestration can use `pin` to preserve
/// the single-pinned-chapter invariant. Workflow phase transitions are
/// handled via [`ChapterStageRepl`] instead.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterPatch {
    /// Unique identifier of the chapter whose profile fields are being changed.
    pub id: String,

    /// New subtitle value, or `None` to leave unchanged.
    pub subtitle: Option<String>,
    /// New pinned state for a dedicated pinning operation.
    pub pin: Option<bool>,
}

impl ChapterPatch {
    /// Borrows normalized subtitle without copying its storage.
    pub fn trimmed_subtitle(&self) -> Option<&str> {
        self.subtitle.as_deref().map(str::trim)
    }
}

/// Mutable workflow stage mask for a chapter.
///
/// The use case layer validates transition legality before building this update.
#[cfg_attr(test, derive(Clone))]
pub struct ChapterStageRepl {
    /// Unique identifier of the chapter whose stages are being transitioned.
    pub id: String,

    /// Updated workflow stage mask after applying the transition.
    pub stages: StageMask,
}
