//! System mail use cases — list unread and mark as read for the current user.

use poprako_util::time::ToUnixMilli;

use crate::data::system_mail::{ListSystemMailData, SystemMailVal};
use crate::model::user::UserToken;
use crate::part::repo::step::system_mail::SystemMailStep;
use crate::part::repo::system_mail::{
    SystemMailRepo, SystemMailRepoTransactional,
};
use crate::result::{RegularResult, accept};
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
) -> RegularResult<Vec<SystemMailVal>>
where
    R: SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: SystemMailRepoTransactional<C>,
{
    let system_mail_infos = repo
        .execute(&SystemMailStep::list_infos(
            &token.user_id,
            data.read,
            data.offset,
            data.limit,
        ))
        .await?;

    let system_mail_vals = system_mail_infos
        .into_iter()
        .map(|system_mail_info| SystemMailVal {
            id: system_mail_info.id,
            title: system_mail_info.title,
            content: system_mail_info.content,
            read: system_mail_info.read,
            created_at: system_mail_info.created_at.to_unix_milli(),
        })
        .collect();

    // FIXME: accept
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
pub async fn mark_read<C, R>(
    repo: &R,
    token: UserToken,
    ids: Vec<String>,
) -> RegularResult<()>
where
    R: SystemMailRepo<C>,
    <R as DeriveTransactional>::Transactional: SystemMailRepoTransactional<C>,
{
    for id in &ids {
        repo.execute(&SystemMailStep::mark_read(id, &token.user_id))
            .await?;
    }

    accept(())
}
