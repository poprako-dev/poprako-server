//! System mail use cases — list unread and mark as read for the current user.

use poprako_util::time::ToUnixMilli;

use crate::data::system_mail::ListSystemMailInfosParams;
use crate::data::system_mail::SystemMailInfoVal;
use crate::model::user::UserToken;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead,
};
use crate::part::repo::system_mail::SystemMailRepo;
use crate::result::RegularResult;

#[cfg(test)]
mod tests;

/// Lists system mails for the current user.
///
/// Non-transactional read — returns mails ordered by creation time
/// descending, filtered and paginated via [`ListSystemMailInfosParams`].
/// The `read` field controls status filtering: [`Some`] returns only
/// matching status; [`None`] returns all.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: SystemMailRepo<C>` — System mail storage.
///
/// [`ListSystemMailInfosParams`]: ListSystemMailInfosParams
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    params: ListSystemMailInfosParams,
) -> RegularResult<Vec<SystemMailInfoVal>>
where
    R: SystemMailRepo<C>,
{
    let system_mail_infos = repo
        .run(&ListSystemMailInfos {
            receiver_id: &token.user_id,
            read: params.read,
            offset: params.offset,
            limit: params.limit,
        })
        .await?;

    let system_mail_vals = system_mail_infos
        .into_iter()
        .map(|system_mail_info| SystemMailInfoVal {
            id: system_mail_info.id,
            title: system_mail_info.title,
            content: system_mail_info.content,
            read: system_mail_info.read,
            created_at: system_mail_info.created_at.to_unix_milli(),
        })
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
pub async fn mark_read<C, R>(
    repo: &R,
    token: UserToken,
    ids: Vec<String>,
) -> RegularResult<()>
where
    R: SystemMailRepo<C>,
{
    for id in &ids {
        repo.run(&MarkSystemMailRead {
            id,
            user_id: &token.user_id,
        })
        .await?;
    }

    Ok(())
}
