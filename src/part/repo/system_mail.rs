//! Repository traits for the system mail domain.

use poprako_orchestra::Run;

use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail, SendSystemMails,
};
use crate::result::RegularError;

/// System mail repository operations.
///
/// Each operation runs independently because the existing catalog contains
/// only single-table reads and writes.
pub trait SystemMailRepo<C>:
    for<'a> Run<SendSystemMail<'a>, Error = RegularError>
    + for<'a> Run<SendSystemMails<'a>, Error = RegularError>
    + for<'a> Run<ListSystemMailInfos<'a>, Error = RegularError>
    + for<'a> Run<MarkSystemMailRead<'a>, Error = RegularError>
{
}
