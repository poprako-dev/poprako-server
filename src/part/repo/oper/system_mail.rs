use poprako_orchestra::Oper;

use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::read::spec::system_mail::SystemMailListSpec;
use crate::model::write::system_mail::SystemMailEntry;

/// Sends one system mail.
#[derive(Oper)]
#[oper(output = ())]
pub struct SendSystemMail<'a> {
    /// The system mail entry to send.
    pub entry: &'a SystemMailEntry,
}

/// Sends a batch of system mails atomically.
#[derive(Oper)]
#[oper(output = ())]
pub struct SendSystemMails<'a> {
    /// The batch of system mail entries to send.
    pub entries: &'a [SystemMailEntry],
}

/// Lists system mail infos for one receiver.
#[derive(Oper)]
#[oper(output = Vec<SystemMailInfo>)]
pub struct ListSystemMailInfos<'a> {
    /// The specification for filtering listed system mails.
    pub spec: &'a SystemMailListSpec,
}

/// Marks one system mail as read after verifying receiver ownership.
#[derive(Oper)]
#[oper(output = ())]
pub struct MarkSystemMailRead<'a> {
    /// The system mail id.
    pub id: &'a str,

    /// The user id of the receiver.
    pub user_id: &'a str,
}
