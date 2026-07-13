//! Repository traits for the team domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::team::{
    AllocateTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, ReserveTeamAvatar, UpdateTeam,
};
use crate::result::RegularError;

/// Team repository operations.
///
/// Standalone reads and updates use [`Run`]. Transactional mutations, locks,
/// and sequence allocation use [`Step`] with the caller-owned context.
pub trait TeamRepo<C>:
    for<'a> Run<CreateTeam<'a>, Error = RegularError>
    + for<'a> Run<GetTeamInfo<'a>, Error = RegularError>
    + for<'a> Run<ListTeamInfos<'a>, Error = RegularError>
    + for<'a> Run<UpdateTeam<'a>, Error = RegularError>
    + for<'a> Step<CreateTeam<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateTeam<'a>, C, Error = RegularError>
    + for<'a> Step<ReserveTeamAvatar<'a>, C, Error = RegularError>
    + for<'a> Step<GetTeamInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteTeam<'a>, C, Error = RegularError>
    + for<'a> Step<AllocateTeamWorksetIndex<'a>, C, Error = RegularError>
{
}
