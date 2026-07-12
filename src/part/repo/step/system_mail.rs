//! Step types for system mail repository opers.

use poprako_transactional::step::Step;

use crate::model::system_mail_model;

/// Step that inserts a single system mail row.
pub struct Send<'a> {
    pub form: &'a system_mail_model::Form,
}

impl<'a> Step for Send<'a> {
    type Output = ();
}

/// Step that inserts multiple system mail rows atomically.
pub struct SendBatch<'a> {
    pub forms: &'a [system_mail_model::Form],
}

impl<'a> Step for SendBatch<'a> {
    type Output = ();
}

/// Step that lists system mails with filters and pagination.
pub struct ListInfosByReceiverId<'a> {
    pub receiver_id: &'a str,

    pub read: Option<bool>,

    pub offset: u32,
    pub limit: u32,
}

impl<'a> Step for ListInfosByReceiverId<'a> {
    type Output = Vec<system_mail_model::Info>;
}

/// Step that marks a system mail as read, verifying receiver ownership.
pub struct MarkRead<'a> {
    pub id: &'a str,

    pub user_id: &'a str,
}

impl<'a> Step for MarkRead<'a> {
    type Output = ();
}

/// Factory for constructing system mail repository [`Step`] values.
pub struct SystemMailStep;

impl SystemMailStep {
    /// Constructs a step to insert a single system mail.
    pub fn send<'a>(form: &'a system_mail_model::Form) -> Send<'a> {
        Send { form }
    }

    /// Constructs a step to insert multiple system mails atomically.
    pub fn send_batch<'a>(
        forms: &'a [system_mail_model::Form],
    ) -> SendBatch<'a> {
        SendBatch { forms }
    }

    /// Constructs a step to list system mails.
    pub fn list_infos<'a>(
        receiver_id: &'a str,
        read: Option<bool>,
        offset: u32,
        limit: u32,
    ) -> ListInfosByReceiverId<'a> {
        ListInfosByReceiverId {
            receiver_id,
            read,
            offset,
            limit,
        }
    }

    /// Constructs a step to mark a system mail as read.
    pub fn mark_read<'a>(id: &'a str, user_id: &'a str) -> MarkRead<'a> {
        MarkRead { id, user_id }
    }
}
