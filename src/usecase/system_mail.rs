//! System mail use cases — list unread and mark as read for the current user.

use poprako_orchestra::OperRun as _;
use tracing::instrument;

use poprako_util::time::ToUnixMilli as _;

use crate::data::instr::system_mail::ListSystemMailInfosInstr;
use crate::data::view::system_mail::SystemMailInfoView;
use crate::model::read::spec::system_mail::SystemMailListSpec;
use crate::model::shared::user::UserToken;
use crate::part::repo::oper::system_mail::{
    ListSystemMailInfos, MarkSystemMailRead,
};
use crate::part::repo::system_mail::SystemMailRepo;
use crate::result::{BaseRest, accept};

#[cfg(test)]
mod tests;

/// Lists system mails for the current user.
///
/// Non-transactional read — returns mails ordered by creation time
/// descending, filtered and paginated via [`ListSystemMailInfosInstr`].
/// The `read` field controls status filtering: [`Some`] returns only
/// matching status; [`None`] returns all.
///
/// # Type Parameters
///
/// * `R: SystemMailRepo` — System mail storage.
///
/// [`ListSystemMailInfosInstr`]: ListSystemMailInfosInstr
#[instrument(level = "info", skip(repo))]
pub async fn list_infos<R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListSystemMailInfosInstr,
) -> BaseRest<Vec<SystemMailInfoView>>
where
    R: SystemMailRepo,
{
    let system_mail_list_spec = SystemMailListSpec {
        receiver_id: token.user_id,
        is_read: instr.is_read,
        offset: instr.offset,
        limit: instr.limit,
    };

    let system_mail_infos = ListSystemMailInfos {
        spec: &system_mail_list_spec,
    }
    .run_on(repo)
    .await?;

    let system_mail_vals = system_mail_infos
        .into_iter()
        .map(|system_mail_info| SystemMailInfoView {
            id: system_mail_info.id,
            title: system_mail_info.title,
            content: system_mail_info.content,
            is_read: system_mail_info.is_read,
            created_at: system_mail_info.created_at.to_unix_milli(),
        })
        .collect();

    accept(system_mail_vals)
}

/// Marks a batch of system mails as read for the current user.
///
/// Non-transactional — first fetches the mails by `ids` to verify
/// ownership, then marks each as read. Returns a perm error
/// if any mail does not belong to the user identified by `token`.
///
/// # Type Parameters
///
/// * `R: SystemMailRepo` — System mail storage.
#[instrument(level = "info", skip(repo))]
pub async fn mark_read<R>(
    (repo,): (&R,),
    token: UserToken,
    ids: Vec<String>,
) -> BaseRest<()>
where
    R: SystemMailRepo,
{
    for id in &ids {
        //
        MarkSystemMailRead {
            id,
            user_id: &token.user_id,
        }
        .run_on(repo)
        .await?;
    }

    accept(())
}
