//! Repository traits for the team domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, LockTeam, ReserveTeamAvatar,
    UpdateTeam,
};
use crate::result::BaseError;

/// Team repository operations.
///
/// Standalone reads and updates use [`poprako_orchestra::Run`]. Transactional mutations, locks,
/// and sequence allocation use [`poprako_orchestra::Step`] with the caller-owned context.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> CreateTeam<'a>,
        for<'a> GetTeamInfo<'a>,
        for<'a> ListTeamInfos<'a>,
        for<'a> UpdateTeam<'a>,
    ),
    step(
        for<'a> CreateTeam<'a>,
        for<'a> UpdateTeam<'a>,
        for<'a> ReserveTeamAvatar<'a>,
        for<'a> GetTeamInfoExcluded<'a>,
        for<'a> LockTeam<'a>,
        for<'a> DeleteTeam<'a>,
        for<'a> AllocTeamWorksetIndex<'a>,
    ),
)]
pub trait TeamRepo<C> {}
