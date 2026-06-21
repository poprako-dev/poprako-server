use poprako_transactional::advance::Advance;

use crate::part::repo::Execute;
use crate::part::repo::step::team::{
    Create, Delete, GetInfoById, GetInfoExcluded, List, MarkAvatarUploaded, ReserveAvatar,
    UpdateInfo,
};
use crate::result::RootError;
use crate::util::DeriveTransactional;

pub trait TeamRepo<C>:
    DeriveTransactional
    + for<'a> Execute<Create<'a>, Error = RootError>
    + for<'a> Execute<GetInfoById<'a>, Error = RootError>
    + Execute<List, Error = RootError>
    + for<'a> Execute<UpdateInfo<'a>, Error = RootError>
    + for<'a> Execute<MarkAvatarUploaded<'a>, Error = RootError>
where
    Self::Transactional: TeamRepoTransactional<C>,
{
}

pub trait TeamRepoTransactional<C>:
    for<'a> Advance<ReserveAvatar<'a>, C, Error = RootError>
    + for<'a> Advance<MarkAvatarUploaded<'a>, C, Error = RootError>
    + for<'a> Advance<GetInfoExcluded<'a>, C, Error = RootError>
    + for<'a> Advance<Delete<'a>, C, Error = RootError>
{
}
