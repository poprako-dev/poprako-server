use poprako_orchestra::Oper;

use crate::model::system_mail::{SystemMailEntry, SystemMailInfo};

/// Sends one system mail.
pub struct SendSystemMail<'a> {
    pub entry: &'a SystemMailEntry,
}

impl Oper for SendSystemMail<'_> {
    type Output = ();
}

/// Sends a batch of system mails atomically.
pub struct SendSystemMails<'a> {
    pub entries: &'a [SystemMailEntry],
}

impl Oper for SendSystemMails<'_> {
    type Output = ();
}

/// Lists system mail infos for one receiver.
pub struct ListSystemMailInfos<'a> {
    pub receiver_id: &'a str,

    pub read: Option<bool>,

    pub offset: u32,
    pub limit: u32,
}

impl Oper for ListSystemMailInfos<'_> {
    type Output = Vec<SystemMailInfo>;
}

/// Marks one system mail as read after verifying receiver ownership.
pub struct MarkSystemMailRead<'a> {
    pub id: &'a str,

    pub user_id: &'a str,
}

impl Oper for MarkSystemMailRead<'_> {
    type Output = ();
}
