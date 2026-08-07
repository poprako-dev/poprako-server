use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::UserAvatarReservation;
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::user::helpers::{
    create, delete, find_info_by_qid, get_credential_by_qid, get_info_by_id,
    get_info_by_id_excluded, mark_avatar_uploaded, reserve_avatar,
    touch_last_active, update_info, update_password_hash,
};
use crate::result::{BaseError, BaseRest};
use crate::shared::RdbContext;

impl Run<GetUserInfo<'_>> for HybRepo {
    // Use `BaseError` for non-transactional repository reads.
    type Error = BaseError;

    // Route read by ID into the shared `submit_query!` orchestration.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserInfo<'_>,
    ) -> Result<UserInfo, Self::Error> {
        //
        match oper {
            //
            GetUserInfo::Id { id } => {
                submit_query!(self.core, get_info_by_id, id)
            }
        }
    }
}

impl Run<GetUserCredential<'_>> for HybRepo {
    // Use `BaseError` for non-transactional credential reads.
    type Error = BaseError;

    // Route credential read by QID to the shared repository query path.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'_>,
    ) -> Result<UserCredential, Self::Error> {
        //
        match oper {
            //
            GetUserCredential::Qid { qid } => {
                submit_query!(self.core, get_credential_by_qid, qid)
            }
        }
    }
}

impl Run<FindUserInfo<'_>> for HybRepo {
    // Use `BaseError` for non-transactional optional reads.
    type Error = BaseError;

    // Route optional user lookup by QID to shared query layer.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &FindUserInfo<'_>,
    ) -> Result<Option<UserInfo>, Self::Error> {
        //
        match oper {
            //
            FindUserInfo::Qid { qid } => {
                submit_query!(self.core, find_info_by_qid, qid)
            }
        }
    }
}

impl Run<UpdateUser<'_>> for HybRepo {
    // Use `BaseError` for non-transactional user mutations.
    type Error = BaseError;

    // Map each update variant to a dedicated helper with explicit argument flow.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateUser<'_>) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateUser::TouchLastActive { id } => {
                submit_query!(self.core, touch_last_active, id)
            }

            UpdateUser::Info { repl } => {
                submit_query!(self.core, update_info, repl)
            }

            UpdateUser::MarkAvatarUploaded { repl } => {
                //
                submit_query!(
                    self.core,
                    mark_avatar_uploaded,
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded
                )
            }

            UpdateUser::PasswordHash { repl } => {
                submit_query!(self.core, update_password_hash, repl)
            }
        }
    }
}

impl Step<CreateUser<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped operations on one repository error type.
    type Error = BaseError;

    // Insert new user rows inside provided transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &CreateUser<'_>,
    ) -> BaseRest<UserInfo> {
        create(context.conn(), oper.entry).await
    }
}

impl Step<FindUserInfo<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped reads on one repository error type.
    type Error = BaseError;

    // Resolve soft-miss lookup inside caller-owned transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &FindUserInfo<'_>,
    ) -> BaseRest<Option<UserInfo>> {
        //
        match oper {
            //
            FindUserInfo::Qid { qid } => {
                find_info_by_qid(context.conn(), qid).await
            }
        }
    }
}

impl Step<UpdateUser<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped updates on one repository error type.
    type Error = BaseError;

    // Dispatch each mutable user operation to one explicit DB helper.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &UpdateUser<'_>,
    ) -> BaseRest<()> {
        //
        match oper {
            //
            UpdateUser::Info { repl } => {
                update_info(context.conn(), repl).await
            }

            UpdateUser::MarkAvatarUploaded { repl } => {
                //
                mark_avatar_uploaded(
                    context.conn(),
                    &repl.id,
                    repl.avatar_version,
                    repl.avatar_key.as_deref(),
                    repl.is_avatar_uploaded,
                )
                .await
            }

            UpdateUser::TouchLastActive { id } => {
                touch_last_active(context.conn(), id).await
            }

            UpdateUser::PasswordHash { repl } => {
                update_password_hash(context.conn(), repl).await
            }
        }
    }
}

impl Step<ReserveUserAvatar<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped reservation on one repository error type.
    type Error = BaseError;

    // Reserve avatar key/version atomically inside the current transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &ReserveUserAvatar<'_>,
    ) -> BaseRest<UserAvatarReservation> {
        //
        reserve_avatar(context.conn(), oper.id, oper.image_hash, oper.image_ext)
            .await
    }
}

impl Step<GetUserInfoExcluded<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped exclusive reads on one repository error type.
    type Error = BaseError;

    // Read user row with lock for callers that mutate next.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &GetUserInfoExcluded<'_>,
    ) -> BaseRest<UserInfo> {
        //
        match oper {
            //
            GetUserInfoExcluded::Id { id } => {
                get_info_by_id_excluded(context.conn(), id).await
            }
        }
    }
}

impl Step<DeleteUser<'_>, RdbContext> for HybRepo {
    // Keep transaction-scoped deletion on one repository error type.
    type Error = BaseError;

    // Execute user deletion as part of ongoing transaction flow.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut RdbContext,
        oper: &DeleteUser<'_>,
    ) -> BaseRest<()> {
        delete(context.conn(), oper.id).await
    }
}
