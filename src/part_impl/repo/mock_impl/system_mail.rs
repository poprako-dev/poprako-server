//! Mock system mail repository operations.

use std::cmp::Reverse;

use poprako_orchestra::Run;

use tracing::instrument;

use crate::model::system_mail::{SystemMailEntry, SystemMailInfo};
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead, SendSystemMail, SendSystemMails,
};
use crate::part::repo::system_mail::SystemMailRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{ExpectedVariant, RegularError, RegularResult};

impl SystemMailRepo<MockContext> for Mock {}

fn insert_mail(state: &mut MockState, entry: &SystemMailEntry) {
    state.system_mails.push(SystemMailInfo {
        id: entry.id.clone(),
        receiver_id: entry.receiver_id.clone(),
        read: false,
        title: entry.title.clone(),
        content: entry.content.clone(),
        created_at: now(),
    });
}

fn send_system_mail(
    state: &mut MockState,
    entry: &SystemMailEntry,
) -> RegularResult<()> {
    //
    if state
        .system_mails
        .iter()
        .any(|system_mail_info| system_mail_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    insert_mail(state, entry);

    Ok(())
}

fn send_system_mails(
    state: &mut MockState,
    entries: &[SystemMailEntry],
) -> RegularResult<()> {
    //
    for system_mail_entry in entries {
        //
        let persisted = state.system_mails.iter().any(|system_mail_info| {
            system_mail_info.id == system_mail_entry.id
        });

        let duplicated = entries
            .iter()
            .filter(|candidate| candidate.id == system_mail_entry.id)
            .count()
            > 1;

        if persisted || duplicated {
            return Err(expected("error-already-exists"));
        }
    }

    for system_mail_entry in entries {
        insert_mail(state, system_mail_entry);
    }

    Ok(())
}

fn list_system_mail_infos(
    state: &MockState,
    oper: &ListSystemMailInfos<'_>,
) -> Vec<SystemMailInfo> {
    //
    let mut system_mail_infos = state
        .system_mails
        .iter()
        .filter(|system_mail_info| {
            system_mail_info.receiver_id == oper.receiver_id
                && match oper.read {
                    //
                    Some(read) => system_mail_info.read == read,

                    None => true,
                }
        })
        .cloned()
        .collect::<Vec<_>>();

    system_mail_infos
        .sort_by_key(|system_mail_info| Reverse(system_mail_info.created_at));

    system_mail_infos
        .into_iter()
        .skip(oper.offset as usize)
        .take(oper.limit as usize)
        .collect()
}

fn mark_system_mail_read(
    state: &mut MockState,
    id: &str,
    user_id: &str,
) -> RegularResult<()> {
    //
    let system_mail_info = state
        .system_mails
        .iter_mut()
        .find(|system_mail_info| system_mail_info.id == id)
        .ok_or_else(|| expected("error-system-mail-not-found"))?;

    if system_mail_info.receiver_id != user_id {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Perm,
            message: "error-forbidden".into(),
        });
    }

    system_mail_info.read = true;

    Ok(())
}

impl Run<SendSystemMail<'_>> for Mock {
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &SendSystemMail<'_>) -> RegularResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        send_system_mail(&mut state, oper.entry)
    }
}

impl Run<SendSystemMails<'_>> for Mock {
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &SendSystemMails<'_>) -> RegularResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        send_system_mails(&mut state, oper.entries)
    }
}

impl Run<ListSystemMailInfos<'_>> for Mock {
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListSystemMailInfos<'_>,
    ) -> RegularResult<Vec<SystemMailInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_system_mail_infos(&state, oper))
    }
}

impl Run<MarkSystemMailRead<'_>> for Mock {
    type Error = RegularError;

#[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &MarkSystemMailRead<'_>) -> RegularResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        mark_system_mail_read(&mut state, oper.id, oper.user_id)
    }
}
