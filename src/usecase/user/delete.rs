use poprako_orchestra::{AtLeast, Context, Nucl, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;
use poprako_util::i18n::trl;

use crate::complex::member as member_complex;
use crate::model::shared::user::UserToken;
use crate::part::nucl::Serial;
use crate::part::obj_dept::UserAvatar;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{
    DeleteUserMemberships, ListMemberInfos, LockTeamMemberInfos,
};
use crate::part::repo::oper::user::{DeleteUser, GetUserInfoExcluded};
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant};

/// Deletes a user and cascades member cleanup.
///
/// * `N: Nucl<Context = C>` — Transaction coordinator.
/// * `C` — Context anchor.
/// * `R: UserRepo<C> + MemberRepo<C>` — User and member storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
/// Deletes a user account and all associated instr.
#[instrument(
    level = "info",
    skip(nucl, repo, obj_dept, token),
    fields(
        actor_user_id = %token.user_id,
    ),
)]
///
/// Transactional cascade:
///
/// 1. **Permission check:** the caller must own the account. Returns `Perm`
///    error on mismatch.
/// 2. Fetches the user info with a pessimistic lock.
pub async fn delete<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: UserRepo<C> + MemberRepo<C> + Send + Sync,
    O: ObjDept<UserAvatar, C> + Send + Sync,
{
    if token.user_id != id {
        //
        return Err(expected_error! {
            ExpectedVariant::Perm,
            message: trl("error-forbidden");
            user_id = %token.user_id,
            affected_user_id = %id,
            "user deletion ownership required",
        });
    }

    nucl.coord(async move |context| {
        //
        GetUserInfoExcluded { id: &id }
            .step_on(repo, context)
            .await?;

        // Delete all memberships before the user to satisfy FK constraints.

        let mut member_infos = ListMemberInfos::User { user_id: &id }
            .step_on(repo, context)
            .await?;

        member_infos.sort_by(|left, right| left.team_id.cmp(&right.team_id));

        for member_info in &member_infos {
            //
            let team_member_infos = LockTeamMemberInfos {
                team_id: &member_info.team_id,
            }
            .step_on(repo, context)
            .await?;

            if !member_complex::team_has_admin_after_delete(
                &team_member_infos,
                member_info,
            ) {
                //
                return Err(expected_error! {
                    ExpectedVariant::Perm,
                    message: trl("error-forbidden");
                    team_id = %member_info.team_id,
                    user_id = %id,
                    member_id = %member_info.id,
                    operation = "delete user holding last team administrator \
                    role",
                    "team administrator retention required",
                });
            }
        }

        DeleteUserMemberships { user_id: &id }
            .step_on(repo, context)
            .await?;

        DeleteObjs::<UserAvatar>::new(std::slice::from_ref(&id))
            .step_on(obj_dept, context)
            .await
            .map_err(BaseError::from)?;

        // FIXME: Clean up tombstone subtree references before deleting the user.
        DeleteUser { id: &id }.step_on(repo, context).await?;

        accept!(())
    })
    .await?;

    accept!(())
}
