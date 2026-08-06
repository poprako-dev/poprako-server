//! RDB-backed member repository — free query functions and thin trait impls.

use poprako_orchestra::Run;
use step_impl::*;
use tracing::instrument;

use crate::model::read::proj::member::MemberInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part_impl::repo::HybRepo;
use crate::result::{BaseError, BaseRest};

// Orchestration logic for member repository operations.
mod orchestra;
// Member step implementation helpers.
mod step_impl;
/// Member RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl Run<FindMemberInfo<'_>> for HybRepo {
    // Error type for the Run trait impl on member lookup.
    type Error = BaseError;

    // Finds a member by user and team IDs within the given operation spec.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &FindMemberInfo<'_>,
    ) -> BaseRest<Option<MemberInfo>> {
        //
        match oper {
            //
            FindMemberInfo::UserTeam { user_id, team_id } => {
                //
                submit_query!(
                    self.core,
                    find_info_by_user_id_and_team_id,
                    user_id,
                    team_id
                )
            }
        }
    }
}
