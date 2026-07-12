//! Step types for the immutable comic archive repository operations.

use poprako_transactional::step::Step;

use crate::model::comic_archive_model;

/// Step that locks every active record required to archive one comic.
pub struct LockSnapshot<'a> {
    pub comic_id: &'a str,
}

impl<'a> Step for LockSnapshot<'a> {
    type Output = comic_archive_model::Snapshot;
}

/// Step that writes archive rows and removes the active comic subtree.
pub struct Commit<'a> {
    pub comic_archive_write: &'a comic_archive_model::Write,
}

impl<'a> Step for Commit<'a> {
    type Output = ();
}

/// Factory for constructing comic archive repository steps.
pub struct ComicArchiveStep;

impl ComicArchiveStep {
    /// Construct a step that locks one active comic archive snapshot.
    pub fn lock_snapshot<'a>(comic_id: &'a str) -> LockSnapshot<'a> {
        LockSnapshot { comic_id }
    }

    /// Construct a step that commits archive rows and deletes source records.
    pub fn commit<'a>(
        comic_archive_write: &'a comic_archive_model::Write,
    ) -> Commit<'a> {
        Commit {
            comic_archive_write,
        }
    }
}
