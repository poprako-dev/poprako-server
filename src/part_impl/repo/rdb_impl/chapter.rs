//! RDB-backed chapter repository.

mod step_impl;

mod orchestra;
#[cfg(all(test, feature = "repo"))]
mod tests;

use crate::part::repo::chapter::ChapterRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;

impl ChapterRepo<RdbContext> for RdbRepo {}
