use crate::part::repo::comic::ComicRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;

mod orchestra;
mod step_impl;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl ComicRepo<RdbContext> for RdbRepo {}
