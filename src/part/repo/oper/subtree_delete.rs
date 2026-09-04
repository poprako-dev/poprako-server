//! Typed operations for deleting hierarchy roots and their descendants.

use poprako_orchestra::Oper;

use crate::model::read::proj::subtree_delete::{
    SubtreeDeleteScope, SubtreeDeleteSweepTarget,
};
use crate::value::subtree_delete::SubtreeSweepLevel;

/// Identifies the root of one hierarchy deletion.
pub enum SubtreeRoot<'a> {
    //
    /// Delete one team hierarchy.
    Team {
        /// Team identifier.
        id: &'a str,
    },

    /// Delete one workset hierarchy.
    Workset {
        /// Workset identifier.
        id: &'a str,
    },

    /// Delete one comic hierarchy.
    Comic {
        /// Comic identifier.
        id: &'a str,
    },

    /// Delete one chapter hierarchy.
    Chapter {
        /// Chapter identifier.
        id: &'a str,
    },
}

/// Locks a hierarchy root and returns only deletion-relevant ancestry.
#[derive(Oper)]
#[oper(output = SubtreeDeleteScope)]
pub struct LockSubtreeDeleteScope<'a> {
    /// Root selected for deletion.
    pub root: SubtreeRoot<'a>,
}

/// Marks a locked hierarchy scope and its aggregate descendants for cleanup.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkSubtree<'a> {
    /// Locked deletion scope.
    pub scope: &'a SubtreeDeleteScope,
}

/// Claims eligible tombstones from one hierarchy level.
#[derive(Oper)]
#[oper(output = Option<SubtreeDeleteSweepTarget>)]
pub struct ClaimSubtreeSweep {
    /// Hierarchy level to claim without falling through to another level.
    pub level: SubtreeSweepLevel,
}

/// Lists Page identifiers owned by one chapter.
#[derive(Oper)]
#[oper(output = Vec<String>)]
pub struct ListSubtreePageIds<'a> {
    /// Chapter identifier.
    pub chapter_id: &'a str,
}

/// Deletes all relational rows belonging to one locked hierarchy scope.
#[derive(Oper)]
#[oper(output = ())]
pub struct DeleteSubtree<'a> {
    /// Locked deletion scope.
    pub scope: &'a SubtreeDeleteScope,
}

/// Physically deletes one claimed tombstone batch and its direct dependants.
#[derive(Oper)]
#[oper(output = ())]
pub struct SweepSubtree<'a> {
    /// Claimed cleanup target.
    pub target: &'a SubtreeDeleteSweepTarget,
}
