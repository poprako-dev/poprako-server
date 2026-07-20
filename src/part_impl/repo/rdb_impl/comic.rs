mod step_impl;

mod orchestra;
#[cfg(all(test, feature = "repo"))]
mod tests;

use crate::part::repo::comic::ComicRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;

impl ComicRepo<RdbContext> for RdbRepo {}
