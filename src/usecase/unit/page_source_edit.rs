//! Shared Unit page edits inside the caller's transaction.

use poprako_orchestra::{AtLeast, Context, OperStep as _};

use crate::complex::unit as unit_complex;
use crate::model::read::proj::page_source::PageSourceUnitScope;
use crate::model::read::proj::unit::UnitCountDelta;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::Serial;
use crate::part::repo::oper::page_source::SetPageSourceUnitCountMetrics;
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page_source::PageSourceRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::BaseRest;

/// Persisted source page changes needed by the caller's completion policy.
pub struct PageSourceEditChanges {
    /// Counter change to aggregate into the owning Chapter.
    count_delta: UnitCountDelta,
    /// Normalized edits actually submitted for persistence.
    applied_edits: Vec<UnitEdit>,
}

impl PageSourceEditChanges {
    /// Returns the counter change for the owning Chapter.
    pub const fn count_delta(&self) -> UnitCountDelta {
        self.count_delta
    }

    /// Consumes the outcome into its Chapter delta and persisted edits.
    pub fn into_changes(self) -> (UnitCountDelta, Vec<UnitEdit>) {
        (self.count_delta, self.applied_edits)
    }
}

/// Applies edits and source page counters using a scope already locked by the caller.
///
/// The caller retains authorization, Chapter-before-source page locking, Chapter
/// aggregation, and workflow policy. Empty sparse write plans still update
/// source page counters; callers decide whether an operation is a no-op before entry.
pub async fn apply_edits<C, R>(
    repo: &R,
    context: &mut C,
    page_source_scope: &PageSourceUnitScope,
    edits: Vec<UnitEdit>,
) -> BaseRest<PageSourceEditChanges>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: PageSourceRepo<C> + UnitRepo<C> + Sync,
{
    let orders = ListUnitOrders {
        page_source_id: &page_source_scope.id,
    }
    .step_on(repo, context)
    .await?;

    let base_ids = orders.iter().map(|order| order.id.as_str());

    let edits = unit_complex::normalize_edits(base_ids, edits)?;

    let count_metrics = ApplyUnitEdits {
        page_source_id: &page_source_scope.id,
        orders: &orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    SetPageSourceUnitCountMetrics {
        id: &page_source_scope.id,
        count_metrics,
    }
    .step_on(repo, context)
    .await?;

    let count_delta =
        page_source_scope.count_metrics.calc_delta(count_metrics)?;

    accept!(PageSourceEditChanges {
        count_delta,
        applied_edits: edits,
    })
}
