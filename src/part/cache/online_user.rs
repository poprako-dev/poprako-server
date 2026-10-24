//! Cache traits for process-local online-user leases.

use poprako_orchestra::drive;

use crate::part::cache::oper::online_user::{
    ListOnlineUserIds, MarkUserOnline,
};
use crate::result::BaseError;

/// Online-user cache adapter operations.
///
/// Both operations run independently because leases are process-local and do
/// not participate in database transactions.
#[drive(
    error = BaseError,
    run(for<'a> MarkUserOnline<'a>, for<'a> ListOnlineUserIds<'a>),
)]
pub trait OnlineUserCache {}
