//! RDB-backed page repository.

use crate::part::repo::page::PageRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;

mod orchestra;
mod step_impl;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

// FIXME: use impl xxx for T
impl PageRepo<RdbContext> for RdbRepo {}
