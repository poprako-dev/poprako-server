//! RDB-backed member repository — free query functions and thin trait impls.

use poprako_orchestra::Run;
use step_impl::*;
use tracing::instrument;

use crate::model::member::MemberInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part_impl::repo::rdb_impl::RdbRepo;
use crate::result::{BaseError, BaseResult};

mod orchestra;
mod step_impl;
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl Run<FindMemberInfo<'_>> for RdbRepo {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindMemberInfo<'_>,
    ) -> BaseResult<Option<MemberInfo>> {
        match oper {
            FindMemberInfo::UserTeam { user_id, team_id } => {
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
