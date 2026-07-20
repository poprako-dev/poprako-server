//! RDB-backed assignment invitation repository.

use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::part_impl::shared::RdbContext;

mod step_impl;

mod orchestra;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl AssignmentInvitationRepo<RdbContext> for RdbRepo {}
