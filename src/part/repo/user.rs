//! Repository traits for the user domain.

use poprako_orchestra::{Run, Step};

use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::result::RegularError;

/// User repository operations.
///
/// Independent reads and activity updates use [`Run`]. Mutations and locks
/// use [`Step`] with the context coordinated by the caller.
pub trait UserRepo<C>:
    for<'a> Run<GetUserInfo<'a>, Error = RegularError>
    + for<'a> Run<GetUserCredential<'a>, Error = RegularError>
    + for<'a> Run<FindUserInfo<'a>, Error = RegularError>
    + for<'a> Run<UpdateUser<'a>, Error = RegularError>
    + for<'a> Step<CreateUser<'a>, C, Error = RegularError>
    + for<'a> Step<FindUserInfo<'a>, C, Error = RegularError>
    + for<'a> Step<UpdateUser<'a>, C, Error = RegularError>
    + for<'a> Step<ReserveUserAvatar<'a>, C, Error = RegularError>
    + for<'a> Step<GetUserInfoExcluded<'a>, C, Error = RegularError>
    + for<'a> Step<DeleteUser<'a>, C, Error = RegularError>
{
}
