//! Mock system mail repository operations.

use std::cmp::Reverse;
use std::collections::HashSet;

use poprako_orchestra::Run;
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::model::read::proj::system_mail::SystemMailInfo;
use crate::model::write::system_mail::SystemMailEntry;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailsRead, SendSystemMail, SendSystemMails,
};
use crate::part_impl::repo::mock_impl::{Mock, MockState, expected, now};
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

// Internal implementation of `insert_mail`.
fn insert_mail(state: &mut MockState, entry: &SystemMailEntry) {
    //
    state.system_mails.push(SystemMailInfo {
        id: entry.id.clone(),
        receiver_id: entry.receiver_id.clone(),
        is_read: false,
        title: entry.title.clone(),
        content: entry.content.clone(),
        created_at: now(),
    });
}

// Internal implementation of `send_system_mail`.
fn send_system_mail(
    state: &mut MockState,
    entry: &SystemMailEntry,
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state
        .system_mails
        .iter()
        .any(|system_mail_info| system_mail_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    insert_mail(state, entry);

    accept(())
}

// Internal implementation of `send_system_mails`.
fn send_system_mails(
    state: &mut MockState,
    entries: &[SystemMailEntry],
) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    for system_mail_entry in entries {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let persisted = state.system_mails.iter().any(|system_mail_info| {
            system_mail_info.id == system_mail_entry.id
        });

        let duplicated = entries
            .iter()
            .filter(|cand| cand.id == system_mail_entry.id)
            .count()
            > 1;

        if persisted || duplicated {
            return Err(expected("error-already-exists"));
        }
    }

    for system_mail_entry in entries {
        insert_mail(state, system_mail_entry);
    }

    accept(())
}

// Internal implementation of `list_system_mail_infos`.
fn list_system_mail_infos(
    state: &MockState,
    oper: &ListSystemMailInfos<'_>,
) -> Vec<SystemMailInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut system_mail_infos = state
        .system_mails
        .iter()
        .filter(|system_mail_info| {
            //
            system_mail_info.receiver_id == oper.spec.receiver_id
                && oper
                    .spec
                    .is_read
                    .map(|is_read| system_mail_info.is_read == is_read)
                    .unwrap_or(true)
        })
        .cloned()
        .collect::<Vec<_>>();

    system_mail_infos
        .sort_by_key(|system_mail_info| Reverse(system_mail_info.created_at));

    system_mail_infos
        .into_iter()
        .skip(oper.spec.offset as usize)
        .take(oper.spec.limit as usize)
        .collect()
}

// Validate a batch of system mails before marking the complete batch as read.
fn mark_system_mails_read(
    state: &mut MockState,
    ids: &[String],
    user_id: &str,
) -> BaseRest<()> {
    //
    for id in ids {
        //
        let system_mail_info = state
            .system_mails
            .iter()
            .find(|system_mail_info| system_mail_info.id == *id)
            .ok_or_else(|| expected("error-system-mail-not-found"))?;

        if system_mail_info.receiver_id != user_id {
            //
            return Err(BaseError::Expected {
                variant: ExpectedVariant::Perm,
                message: trl("error-forbidden"),
            });
        }
    }

    let system_mail_ids =
        ids.iter().map(String::as_str).collect::<HashSet<_>>();

    for system_mail_info in &mut state.system_mails {
        //
        if system_mail_ids.contains(system_mail_info.id.as_str()) {
            system_mail_info.is_read = true;
        }
    }

    accept(())
}

impl Run<SendSystemMail<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &SendSystemMail<'_>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        send_system_mail(&mut state, oper.entry)
    }
}

impl Run<SendSystemMails<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &SendSystemMails<'_>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        send_system_mails(&mut state, oper.entries)
    }
}

impl Run<ListSystemMailInfos<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListSystemMailInfos<'_>,
    ) -> BaseRest<Vec<SystemMailInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_system_mail_infos(&state, oper))
    }
}

impl Run<MarkSystemMailsRead<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &MarkSystemMailsRead<'_>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        mark_system_mails_read(&mut state, oper.ids, oper.user_id)
    }
}
