//! RDB-backed team repository — free query functions and thin trait impls.

// RDB team-ownership projections.
mod resolve;
// Team free-function helpers.
mod helpers;
// Team repository impl blocks.
mod impls;

/// Team RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::team::TeamInfo;
use crate::part::repo::oper::team::{CreateTeam, GetTeamInfo, ListTeamInfos};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::team::helpers::{
    create, get_info_by_id, list_infos,
};
use crate::result::BaseError;

impl<'a> Run<CreateTeam<'a>> for HybRepo {
    // Map team creation orchestration failures to the shared base error type.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Submit a team create request through repository core to keep one call path.
    async fn run(
        &self,
        oper: &CreateTeam<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        submit_query!(self.core, create, oper.entry)
    }
}

impl Run<GetTeamInfo<'_>> for HybRepo {
    // Use the common base error for team info reads.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Resolve team read requests from ID-based variants and return team details.
    async fn run(
        &self,
        oper: &GetTeamInfo<'_>,
    ) -> Result<TeamInfo, Self::Error> {
        //
        match oper {
            //
            GetTeamInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl Run<ListTeamInfos<'_>> for HybRepo {
    // Keep list query failures on a single repository-level error channel.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Return filtered and paginated team lists based on caller-provided criteria.
    async fn run(
        &self,
        oper: &ListTeamInfos<'_>,
    ) -> Result<Vec<TeamInfo>, Self::Error> {
        submit_query!(self.core, list_infos, oper.spec)
    }
}
