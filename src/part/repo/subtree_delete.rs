//! Repository capability for atomic hierarchy deletion.

#![allow(
    clippy::trait_duplication_in_bounds,
    reason = "drive generates repeated bounds for each transaction step"
)]

use poprako_orchestra::drive;

use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, DeleteSubtree, ListSubtreePageIds,
    LockSubtreeDeleteScope, MarkSubtree, SweepSubtree,
};
use crate::result::BaseError;

/// Transaction-only hierarchy deletion operations.
#[drive(
    context = C,
    error = BaseError,
    step(
        for<'a> LockSubtreeDeleteScope<'a>,
        for<'a> MarkSubtree<'a>,
        ClaimSubtreeSweep,
        for<'a> ListSubtreePageIds<'a>,
        for<'a> DeleteSubtree<'a>,
        for<'a> SweepSubtree<'a>,
    ),
)]
pub trait SubtreeRepo<C> {}
