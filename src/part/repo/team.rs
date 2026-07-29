//! Repository traits for the team domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::team::{
    AllocTeamWorksetIndex, CreateTeam, DeleteTeam, GetTeamInfo,
    GetTeamInfoExcluded, ListTeamInfos, ReserveTeamAvatar, UpdateTeam,
};
use crate::result::BaseError;

/// Team repository operations.
///
/// Standalone reads and updates use [`Run`]. Transactional mutations, locks,
/// and sequence allocation use [`Step`] with the caller-owned context.
pub trait TeamRepo<C>:
    for<'a> Run<CreateTeam<'a>, Error = BaseError>
    + for<'a> Run<GetTeamInfo<'a>, Error = BaseError>
    + for<'a> Run<ListTeamInfos<'a>, Error = BaseError>
    + for<'a> Run<UpdateTeam<'a>, Error = BaseError>
    + for<'a> Step<CreateTeam<'a>, C, Error = BaseError>
    + for<'a> Step<UpdateTeam<'a>, C, Error = BaseError>
    + for<'a> Step<ReserveTeamAvatar<'a>, C, Error = BaseError>
    + for<'a> Step<GetTeamInfoExcluded<'a>, C, Error = BaseError>
    + for<'a> Step<DeleteTeam<'a>, C, Error = BaseError>
    + for<'a> Step<AllocTeamWorksetIndex<'a>, C, Error = BaseError>
{
}

impl<T, C> TeamRepo<C> for T where
    T: for<'a> Run<CreateTeam<'a>, Error = BaseError>
        + for<'a> Run<GetTeamInfo<'a>, Error = BaseError>
        + for<'a> Run<ListTeamInfos<'a>, Error = BaseError>
        + for<'a> Run<UpdateTeam<'a>, Error = BaseError>
        + for<'a> Step<CreateTeam<'a>, C, Error = BaseError>
        + for<'a> Step<UpdateTeam<'a>, C, Error = BaseError>
        + for<'a> Step<ReserveTeamAvatar<'a>, C, Error = BaseError>
        + for<'a> Step<GetTeamInfoExcluded<'a>, C, Error = BaseError>
        + for<'a> Step<DeleteTeam<'a>, C, Error = BaseError>
        + for<'a> Step<AllocTeamWorksetIndex<'a>, C, Error = BaseError>
{
}
