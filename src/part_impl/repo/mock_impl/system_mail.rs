//! Mock implementations of `SystemMailRepo` and `SystemMailRepoTransactional` for in-memory
//! testing.

use async_trait::async_trait;
use time::OffsetDateTime;

use crate::model::system_mail::{SystemMailForm, SystemMailInfo};
use crate::part::repo::step::system_mail::{
    ListInfosByReceiverId, MarkRead, Send, SendBatch,
};
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::{ExpectedVariant, RegularError};

impl SystemMailRepo<MockContext> for Mock {}

impl SystemMailRepoTransactional<MockContext> for MockTransactional {}

/// Appends a new system mail as unread to the in-memory store.
fn insert_mail(state: &mut MockState, form: &SystemMailForm) {
    state.system_mails.push(SystemMailInfo {
        id: form.id.clone(),
        receiver_id: form.receiver_id.clone(),
        read: false,
        title: form.title.clone(),
        content: form.content.clone(),
        created_at: now(),
    });
}

#[async_trait]
impl<'a> Execute<Send<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &Send<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();

        if state
            .system_mails
            .iter()
            .any(|mail| mail.id == step.form.id)
        {
            return Err(expected("error-already-exists"));
        }

        insert_mail(&mut state, step.form);

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<SendBatch<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &SendBatch<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();

        for system_mail_form in step.forms {
            if state
                .system_mails
                .iter()
                .any(|system_mail| system_mail.id == system_mail_form.id)
            {
                return Err(expected("error-already-exists"));
            }

            let duplicate_in_batch = step
                .forms
                .iter()
                .filter(|candidate| candidate.id == system_mail_form.id)
                .count()
                > 1;

            if duplicate_in_batch {
                return Err(expected("error-already-exists"));
            }
        }

        for system_mail_form in step.forms {
            insert_mail(&mut state, system_mail_form);
        }

        Ok(())
    }
}

#[async_trait]
impl<'a> Execute<ListInfosByReceiverId<'a>> for Mock {
    type Error = RegularError;

    async fn execute(
        &self,
        step: &ListInfosByReceiverId<'a>,
    ) -> Result<Vec<SystemMailInfo>, Self::Error> {
        let state = self.state.lock().unwrap();

        let mut mails: Vec<SystemMailInfo> = state
            .system_mails
            .iter()
            .filter(|mail| {
                mail.receiver_id == step.receiver_id
                    && match step.read {
                        Some(expected) => mail.read == expected,
                        None => true,
                    }
            })
            .cloned()
            .collect();

        mails.sort_by_key(|b| std::cmp::Reverse(b.created_at));

        Ok(mails
            .into_iter()
            .skip(step.offset as usize)
            .take(step.limit as usize)
            .collect())
    }
}

#[async_trait]
impl<'a> Execute<MarkRead<'a>> for Mock {
    type Error = RegularError;

    async fn execute(&self, step: &MarkRead<'a>) -> Result<(), Self::Error> {
        let mut state = self.state.lock().unwrap();

        let mail = state
            .system_mails
            .iter_mut()
            .find(|mail| mail.id == step.id)
            .ok_or_else(|| expected("error-system-mail-not-found"))?;

        if mail.receiver_id != step.user_id {
            return Err(RegularError::Expected {
                variant: ExpectedVariant::Perm,
                message: "error-forbidden".into(),
            });
        }

        mail.read = true;

        Ok(())
    }
}

// ── Tests ──────────────────────────────────────────────────────────────────

/// send_saves_unread_mail(Send)(positive): a sent mail should be persisted with read=false.
/// send_rejects_duplicate_id_without_mutation(Send)(negative): sending a mail with an existing id should error without altering state.
/// send_batch_saves_all_mails(SendBatch)(positive): a batch should persist all mails atomically.
/// send_batch_rejects_duplicate_batch_without_partial_write(SendBatch)(negative): a batch with a duplicate batch-internal id should reject with no writes.
/// list_infos_by_receiver_id_filters_sorts_and_pages(ListInfos)(positive): should filter by receiver+read, sort by created_at desc, and paginate.
/// mark_read_marks_by_id(MarkRead)(positive): marking by id should set read=true when receiver matches.
/// mark_read_rejects_missing_id(MarkRead)(negative): a nonexistent id should return an argument error.
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::test_util::assert_expected_variant;

fn mail_info(
    id: &str,
    receiver_id: &str,
    read: bool,
    created_at: OffsetDateTime,
) -> SystemMailInfo {
    SystemMailInfo {
        id: id.into(),
        receiver_id: receiver_id.into(),
        read,
        title: "title".into(),
        content: "content".into(),
        created_at,
    }
}

fn mail_form(id: &str, receiver_id: &str) -> SystemMailForm {
    SystemMailForm {
        id: id.into(),
        receiver_id: receiver_id.into(),
        title: "title".into(),
        content: "content".into(),
    }
}

#[tokio::test]
async fn send_saves_unread_mail() {
    let mock = Mock::new();
    let system_mail_form = mail_form("sys_mail-1", "user-1");

    mock.execute(&SystemMailStep::send(&system_mail_form))
        .await
        .unwrap();

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.system_mails.len(), 1);
    assert_eq!(snapshot.system_mails[0].id, "sys_mail-1");
    assert!(!snapshot.system_mails[0].read);
}

#[tokio::test]
async fn send_rejects_duplicate_id_without_mutation() {
    let mock = Mock::new();
    let time = now();
    mock.seed_system_mail(mail_info("sys_mail-1", "user-1", false, time));

    let system_mail_form = mail_form("sys_mail-1", "user-2");

    let err = mock
        .execute(&SystemMailStep::send(&system_mail_form))
        .await
        .err()
        .unwrap();
    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();
    assert_eq!(snapshot.system_mails.len(), 1);
    assert_eq!(snapshot.system_mails[0].receiver_id, "user-1");
}

#[tokio::test]
async fn send_batch_saves_all_mails() {
    let mock = Mock::new();
    let system_mail_forms = vec![
        mail_form("sys_mail-1", "user-1"),
        mail_form("sys_mail-2", "user-2"),
    ];

    mock.execute(&SystemMailStep::send_batch(&system_mail_forms))
        .await
        .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 2);
    assert_eq!(snapshot.system_mails[0].receiver_id, "user-1");
    assert_eq!(snapshot.system_mails[1].receiver_id, "user-2");
}

#[tokio::test]
async fn send_batch_rejects_duplicate_batch_without_partial_write() {
    let mock = Mock::new();
    let system_mail_forms = vec![
        mail_form("sys_mail-1", "user-1"),
        mail_form("sys_mail-1", "user-2"),
    ];

    let err = mock
        .execute(&SystemMailStep::send_batch(&system_mail_forms))
        .await
        .err()
        .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.system_mails.len(), 0);
}

#[tokio::test]
async fn list_infos_by_receiver_id_filters_sorts_and_pages() {
    let mock = Mock::new();
    let t1 = now();
    let t2 = t1 + time::Duration::seconds(10);
    let t3 = t2 + time::Duration::seconds(10);

    mock.seed_system_mail(mail_info("sys_mail-1", "user-1", false, t1));
    mock.seed_system_mail(mail_info("sys_mail-2", "user-1", false, t3));
    mock.seed_system_mail(mail_info("sys_mail-3", "user-1", true, t2));
    mock.seed_system_mail(mail_info("sys_mail-4", "user-2", false, t2));

    let mails = mock
        .execute(&SystemMailStep::list_infos("user-1", Some(false), 0, 10))
        .await
        .unwrap();

    assert_eq!(mails.len(), 2);
    assert_eq!(mails[0].id, "sys_mail-2");
    assert_eq!(mails[1].id, "sys_mail-1");
}

#[tokio::test]
async fn mark_read_marks_by_id() {
    let mock = Mock::new();
    let time = now();
    mock.seed_system_mail(mail_info("sys_mail-1", "user-1", false, time));

    mock.execute(&SystemMailStep::mark_read("sys_mail-1", "user-1"))
        .await
        .unwrap();

    let snapshot = mock.snapshot();
    assert!(snapshot.system_mails[0].read);
}

#[tokio::test]
async fn mark_read_rejects_missing_id() {
    let mock = Mock::new();

    let err = mock
        .execute(&SystemMailStep::mark_read("sys_mail-nonexistent", "user-1"))
        .await
        .err()
        .unwrap();
    assert_expected_variant(err, ExpectedVariant::Args);
}
