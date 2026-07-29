//! Repository traits for the system mail domain.

use poprako_orchestra::Run;

use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail, SendSystemMails,
};
use crate::result::BaseError;

/// System mail repository operations.
///
/// Each operation runs independently because the existing catalog contains
/// only single-table reads and writes.
pub trait SystemMailRepo<C>:
    for<'a> Run<SendSystemMail<'a>, Error = BaseError>
    + for<'a> Run<SendSystemMails<'a>, Error = BaseError>
    + for<'a> Run<ListSystemMailInfos<'a>, Error = BaseError>
    + for<'a> Run<MarkSystemMailRead<'a>, Error = BaseError>
{
}
