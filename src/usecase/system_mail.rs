//! System mail use cases — list unread and mark as read for the current user.

use poprako_util::i18n::trl;

use crate::data::system_mail::{ListSystemMailData, SystemMailVal};
use crate::model::system_mail::SystemMailListSpec;
use crate::model::user::UserToken;
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::repo::system_mail::{SystemMailRepo, SystemMailRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub(crate) mod tests;

/// Lists system mails for the current user.
///
/// Non-transactional read — returns mails ordered by creation time
/// descending, filtered and paginated via [`ListSystemMailData`].
/// The `read` field controls status filtering: [`Some`] returns only
/// matching status; [`None`] returns all.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: SystemMailRepo<C>` — System mail storage.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListSystemMailData,
) -> RootResult<Vec<SystemMailVal>>
where
    R: SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: SystemMailRepoTransactional<C>,
{
    let mail_list_spec = SystemMailListSpec {
        read: data.read,
        page: data.page,
    };

    let system_mail_infos = repo
        .execute(&SystemMailStep::list_by_receiver_id(
            &token.user_id,
            &mail_list_spec,
        ))
        .await?;

    let system_mail_vals = system_mail_infos
        .into_iter()
        .map(SystemMailVal::from_model)
        .collect();

    Ok(system_mail_vals)
}

/// Marks a batch of system mails as read for the current user.
///
/// Non-transactional — first fetches the mails by `ids` to verify
/// ownership, then marks each as read. Returns a permission error
/// if any mail does not belong to the user identified by `token`.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: SystemMailRepo<C>` — System mail storage.
pub async fn mark_read<C, R>(repo: &R, token: UserToken, ids: Vec<String>) -> RootResult<()>
where
    R: SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: SystemMailRepoTransactional<C>,
{
    let system_mail_infos =
        repo.execute(&SystemMailStep::list_by_ids(&ids)).await?;

    if system_mail_infos.len() != ids.len() {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Args,
            message: trl("error-system-mail-not-found"),
        });
    }

    if system_mail_infos
        .iter()
        .any(|system_mail_info| system_mail_info.receiver_id != token.user_id)
    {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-forbidden"),
        });
    }

    for id in &ids {
        repo.execute(&SystemMailStep::mark_read(id)).await?;
    }

    accept(())
}
