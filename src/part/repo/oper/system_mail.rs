use poprako_orchestra::Oper;

use crate::model::system_mail::{
    SystemMailEntry, SystemMailInfo, SystemMailInfoListSpec,
};

/// Sends one system mail.
pub struct SendSystemMail<'a> {
    /// The system mail entry to send.
    pub entry: &'a SystemMailEntry,
}

impl Oper for SendSystemMail<'_> {
    // Operation output type.
    type Output = ();
}

/// Sends a batch of system mails atomically.
pub struct SendSystemMails<'a> {
    /// The batch of system mail entries to send.
    pub entries: &'a [SystemMailEntry],
}

impl Oper for SendSystemMails<'_> {
    // Operation output type.
    type Output = ();
}

/// Lists system mail infos for one receiver.
pub struct ListSystemMailInfos<'a> {
    /// The specification for filtering listed system mails.
    pub spec: &'a SystemMailInfoListSpec,
}

impl Oper for ListSystemMailInfos<'_> {
    // Operation output type.
    type Output = Vec<SystemMailInfo>;
}

/// Marks one system mail as read after verifying receiver ownership.
pub struct MarkSystemMailRead<'a> {
    //
    /// The system mail id.
    pub id: &'a str,

    /// The user id of the receiver.
    pub user_id: &'a str,
}

impl Oper for MarkSystemMailRead<'_> {
    // Operation output type.
    type Output = ();
}
