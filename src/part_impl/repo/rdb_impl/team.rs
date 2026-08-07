//! RDB-backed team repository — free query functions and thin trait impls.

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Run, Step};
use time::OffsetDateTime;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::team::TeamComplex;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::write::team::{TeamAvatarReservation, TeamEntry, TeamRepl};
use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, LockTeam, ReserveTeamAvatar,
    UpdateTeam,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::team::{
    TeamAspectRow, TeamEntryRow, TeamInfoRow,
};
use crate::part_impl::repo::rdb_impl::schema::t_member;
use crate::part_impl::repo::rdb_impl::schema::t_team::dsl::*;
use crate::part_impl::repo::rdb_impl::team::helpers::{
    create, get_info_by_id, list_infos,
};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::shared::result::{diesel, next_version};
use crate::shared::{RdbConn, RdbContext};
use crate::value::image::{ImageExt, ImageHash};

// RDB team-ownership projections.
mod resolve;
// Team free-function helpers.
mod helpers;
// Team repository impl blocks.
mod impls;

/// Team RDB integration tests.
#[cfg(all(test, feature = "rdb", feature = "repo_impl"))]
pub mod tests;

impl<'a> Run<CreateTeam<'a>> for HybRepo {
    // Map team creation orchestration failures to the shared base error type.
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
