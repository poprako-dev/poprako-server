use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::image::ImageComplex;
use crate::model::shared::user::UserToken;
use crate::part::nucl::RepeatableRead;
use crate::part::prom::Prom;
use crate::part::prom::oper::Defer;
use crate::part::prom::payload::{TaskPayload, image};
use crate::part::prom::task::Task;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{DeleteMember, ListMemberInfosExcluded};
use crate::part::repo::oper::user::{DeleteUser, GetUserInfoExcluded};
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

/// Deletes a user and cascades member cleanup.
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
#[instrument(level = "info", skip(nucl, repo, prom))]
/// Deletes a user account and all associated instr.
///
/// Transactional cascade:
///
/// 1. **Permission check:** the caller must own the account. Returns `Perm`
///    error on mismatch.
/// 2. Fetches the user info with a pessimistic lock.
pub async fn delete<N, C, R, P>(
    (nucl, repo, prom): (&N, &R, &P),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context,
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    C::Level: AtLeast<RepeatableRead>,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    P: Prom<C> + Send + Sync,
{
    if token.user_id != id {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %id,
            "expected error: user deletion ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    nucl.coord(async move |context| {
        //
        let user_info = GetUserInfoExcluded::Id { id: &id }
            .step_on(repo, context)
            .await?;

        // Delete all memberships before the user to satisfy FK constraints.

        let member_infos = ListMemberInfosExcluded::User { user_id: &id }
            .step_on(repo, context)
            .await?;

        for member_info in &member_infos {
            //
            DeleteMember {
                id: &member_info.id,
            }
            .step_on(repo, context)
            .await?;
        }

        DeleteUser { id: &id }.step_on(repo, context).await?;

        // Enqueue avatar object deletion if one was uploaded.
        if let Some(avatar_key) = &user_info.avatar_key
            && user_info.is_avatar_uploaded == Some(true)
        {
            let (delete_id, payload) = (
                ImageComplex::gen_delete_id(),
                TaskPayload::Image(image::ImagePayload::Delete {
                    object_key: avatar_key.clone(),
                }),
            );

            let task = Task {
                id: &delete_id,
                payload: &payload,
                delay: None,
            };

            Defer::new(task).step_on(prom, context).await?;
        }

        accept(())
    })
    .await?;

    accept(())
}
