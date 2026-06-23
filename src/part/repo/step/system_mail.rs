//! Step types for system mail repository operations.

use poprako_transactional::step::Step;

use crate::model::system_mail::{SystemMailForm, SystemMailInfo, SystemMailListSpec};

/// Step that inserts a single system mail row.
pub struct Send<'a> {
    pub form: &'a SystemMailForm,
}

impl<'a> Step for Send<'a> {
    type Output = ();
}

/// Step that inserts multiple system mail rows atomically.
pub struct SendBatch<'a> {
    pub forms: &'a [SystemMailForm],
}

impl<'a> Step for SendBatch<'a> {
    type Output = ();
}

/// Step that lists system mails for a given receiver, ordered by
/// creation time descending with pagination and optional read filter.
pub struct ListByReceiverId<'a> {
    pub receiver_id: &'a str,
    pub spec: &'a SystemMailListSpec,
}

impl<'a> Step for ListByReceiverId<'a> {
    type Output = Vec<SystemMailInfo>;
}

/// Step that fetches system mails by a batch of identifiers.
pub struct ListByIds<'a> {
    pub ids: &'a [String],
}

impl<'a> Step for ListByIds<'a> {
    type Output = Vec<SystemMailInfo>;
}

/// Step that marks a system mail as read by its identifier.
pub struct MarkRead<'a> {
    pub id: &'a str,
}

impl<'a> Step for MarkRead<'a> {
    type Output = ();
}

/// Factory for constructing system mail repository [`Step`] values.
pub struct SystemMailStep;

impl SystemMailStep {
    /// Constructs a step to insert a single system mail.
    pub fn send<'a>(form: &'a SystemMailForm) -> Send<'a> {
        Send { form }
    }

    /// Constructs a step to insert multiple system mails atomically.
    pub fn send_batch<'a>(forms: &'a [SystemMailForm]) -> SendBatch<'a> {
        SendBatch { forms }
    }

    /// Constructs a step to list system mails for a receiver.
    pub fn list_by_receiver_id<'a>(
        receiver_id: &'a str,
        spec: &'a SystemMailListSpec,
    ) -> ListByReceiverId<'a> {
        ListByReceiverId { receiver_id, spec }
    }

    /// Constructs a step to fetch system mails by a batch of identifiers.
    pub fn list_by_ids<'a>(ids: &'a [String]) -> ListByIds<'a> {
        ListByIds { ids }
    }

    /// Constructs a step to mark a system mail as read.
    pub fn mark_read<'a>(id: &'a str) -> MarkRead<'a> {
        MarkRead { id }
    }
}
