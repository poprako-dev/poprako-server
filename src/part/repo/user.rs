//! Repository traits for the user domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::result::BaseError;

/// User repository operations.
///
/// Independent reads and activity updates use [`poprako_orchestra::Run`]. Mutations and locks
/// use [`poprako_orchestra::Step`] with the context coordinated by the caller.
#[drive(
    context = C,
    error = BaseError,
    run(
        for<'a> GetUserInfo<'a>,
        for<'a> GetUserCredential<'a>,
        for<'a> FindUserInfo<'a>,
        for<'a> UpdateUser<'a>,
    ),
    step(
        for<'a> CreateUser<'a>,
        for<'a> FindUserInfo<'a>,
        for<'a> UpdateUser<'a>,
        for<'a> ReserveUserAvatar<'a>,
        for<'a> GetUserInfoExcluded<'a>,
        for<'a> DeleteUser<'a>,
    ),
)]
pub trait UserRepo<C> {}
