//! Stage selections for correlated Comic filtering.

use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};

/// Normalized stage-filter state.
pub enum StageFilter {
    //
    /// The mask ignores every stage and therefore adds no predicate.
    None,

    /// The mask asks for a phase unsupported by its selected stage.
    Impossible,

    /// Every selected stage and phase must match the same pinned Chapter.
    Mask {
        /// Stage mask used to build the Chapter predicate.
        mask: StageMask,
    },
}

/// Normalizes a stage mask before constructing its typed SQL predicates.
pub fn stage_filter(stage_mask: StageMask) -> StageFilter {
    //
    let stages = StageMask::stages()
        .iter()
        .copied()
        .filter(|stage| !stage_mask.ignores_stage(*stage))
        .map(|stage| (stage, stage_mask.get_phase(stage)))
        .collect::<Vec<_>>();

    if stages.is_empty() {
        return StageFilter::None;
    }

    let has_unsupported_active = stages.iter().any(|(stage, phase)| {
        //
        matches!(
            (stage, phase),
            (
                Stage::RawProvide | Stage::Review | Stage::Publish,
                StagePhase::Active,
            )
        )
    });

    match has_unsupported_active.then_some(()) {
        //
        Some(()) => StageFilter::Impossible,

        None => StageFilter::Mask { mask: stage_mask },
    }
}
