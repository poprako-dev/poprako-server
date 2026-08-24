//! Repository traits for the system mail domain.

use poprako_orchestra::drive;

use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailsRead, SendSystemMail, SendSystemMails,
};
use crate::result::BaseError;

/// System mail repository operations.
///
/// Each operation runs independently because the existing catalog contains
/// only single-table reads and writes.
#[drive(
    error = BaseError,
    run(
        for<'a> SendSystemMail<'a>,
        for<'a> SendSystemMails<'a>,
        for<'a> ListSystemMailInfos<'a>,
        for<'a> MarkSystemMailsRead<'a>,
    ),
)]
pub trait SystemMailRepo {}
