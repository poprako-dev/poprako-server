//! Shared Unit page edits inside the caller's transaction.

use poprako_orchestra::{AtLeast, Context, OperStep as _};

use crate::complex::unit as unit_complex;
use crate::model::read::proj::page::PageUnitScope;
use crate::model::read::proj::unit::UnitCountDelta;
use crate::model::write::unit::UnitEdit;
use crate::part::nucl::Serial;
use crate::part::repo::oper::page::SetPageUnitCountMetrics;
use crate::part::repo::oper::unit::{ApplyUnitEdits, ListUnitOrders};
use crate::part::repo::page::PageRepo;
use crate::part::repo::unit::UnitRepo;
use crate::result::{BaseRest, accept};

/// Persisted Page changes needed by the caller's completion policy.
pub struct PageEditOutcome {
    /// Counter change to aggregate into the owning Chapter.
    count_delta: UnitCountDelta,
    /// Normalized edits actually submitted for persistence.
    applied_edits: Vec<UnitEdit>,
}

impl PageEditOutcome {
    /// Returns the counter change for the owning Chapter.
    pub const fn count_delta(&self) -> UnitCountDelta {
        self.count_delta
    }

    /// Consumes the outcome into its Chapter delta and persisted edits.
    pub fn into_changes(self) -> (UnitCountDelta, Vec<UnitEdit>) {
        (self.count_delta, self.applied_edits)
    }
}

/// Applies edits and Page counters using a scope already locked by the caller.
///
/// The caller retains authorization, Chapter-before-Page locking, Chapter
/// aggregation, and workflow policy. Empty sparse write plans still update
/// Page counters; callers decide whether an operation is a no-op before entry.
pub async fn apply_edits<C, R>(
    repo: &R,
    context: &mut C,
    page_scope: &PageUnitScope,
    edits: Vec<UnitEdit>,
) -> BaseRest<PageEditOutcome>
where
    C: Context + Send,
    C::Level: AtLeast<Serial>,
    R: PageRepo<C> + UnitRepo<C> + Sync,
{
    let orders = ListUnitOrders {
        page_id: &page_scope.id,
    }
    .step_on(repo, context)
    .await?;

    let base_ids = orders
        .iter()
        .map(|order| order.id.as_str())
        .collect::<Vec<_>>();

    let edits = unit_complex::normalize_edits(&base_ids, edits)?;

    let count_metrics = ApplyUnitEdits {
        page_id: &page_scope.id,
        orders: &orders,
        edits: &edits,
    }
    .step_on(repo, context)
    .await?;

    SetPageUnitCountMetrics {
        id: &page_scope.id,
        count_metrics,
    }
    .step_on(repo, context)
    .await?;

    let count_delta = page_scope.count_metrics.calc_delta(count_metrics)?;

    accept(PageEditOutcome {
        count_delta,
        applied_edits: edits,
    })
}
