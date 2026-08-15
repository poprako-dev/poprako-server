//! Mock user repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::complex::user::UserComplex;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::write::user::{UserAvatarReservation, UserEntry};
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Insert a new user entry into mock state and mirror it into credentials.
fn create_user(state: &mut MockState, entry: &UserEntry) -> BaseRest<UserInfo> {
    //
    // Validate unique id/qid before inserting to avoid inconsistent fixture state.
    if state.users.iter().any(|user| user.id == entry.id)
        || state.users.iter().any(|user| user.qid == entry.qid)
    {
        return Err(expected("error-already-exists"));
    }

    let time = now();

    let user_info = UserInfo {
        id: entry.id.clone(),
        qid: entry.qid.clone(),
        nickname: entry.nickname.clone(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    };

    state.users.push(user_info.clone());

    state.credentials.push(UserCredential {
        user_id: entry.id.clone(),
        password_hash: entry.password_hash.clone(),
    });

    accept(user_info)
}

// Load one user info from mock state by id.
fn get_user_info(state: &MockState, id: &str) -> BaseRest<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-user-not-found"))
}

// Find one user info by qid and return an optional result.
fn find_user_info(state: &MockState, qid: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.qid == qid)
        .cloned()
}

// Resolve credential for a qid from linked mock tables.
fn get_user_credential(
    state: &MockState,
    qid: &str,
) -> BaseRest<UserCredential> {
    //
    // Resolve user first, then locate corresponding credential row.
    let user_info = state
        .users
        .iter()
        .find(|user_info| user_info.qid == qid)
        .ok_or_else(|| expected("error-user-not-found"))?;

    state
        .credentials
        .iter()
        .find(|credential| credential.user_id == user_info.id)
        .cloned()
        .ok_or_else(|| expected("error-user-not-found"))
}

// Apply a domain update mutation to the in-memory user store.
fn update_user(state: &mut MockState, oper: &UpdateUser<'_>) -> BaseRest<()> {
    //
    // Dispatch variant to a single mutable flow with optional identity/hash branches.
    let id = match oper {
        //
        UpdateUser::Info { repl } => repl.id.as_str(),

        UpdateUser::MarkAvatarUploaded { repl } => repl.id.as_str(),

        UpdateUser::TouchLastActive { id } => id,

        UpdateUser::PasswordHash { repl } => repl.id.as_str(),
    };

    let update = match oper {
        //
        UpdateUser::Info { repl } => {
            Some((repl.qid.as_str(), repl.nickname.as_str(), None))
        }

        UpdateUser::MarkAvatarUploaded { repl } => Some((
            repl.id.as_str(),
            repl.id.as_str(),
            Some(repl.avatar_version),
        )),

        UpdateUser::TouchLastActive { .. }
        | UpdateUser::PasswordHash { .. } => None,
    };

    let user_info = state
        .users
        .iter_mut()
        .find(|user_info| user_info.id == id)
        .ok_or_else(|| expected("error-user-not-found"))?;

    match update {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        Some((qid, nickname, None)) => {
            //
            // Update qid and nickname on one mutable user object.
            user_info.qid = qid.to_string();

            user_info.nickname = nickname.to_string();
        }

        Some((_, _, Some(avatar_version))) => {
            //
            // Validate optimistic avatar token before toggling upload state.
            if user_info.avatar_version != Some(avatar_version)
                || matches!(
                    oper,
                    UpdateUser::MarkAvatarUploaded { repl }
                        if repl.avatar_key.as_deref().is_some_and(|avatar_key| {
                            user_info.avatar_key.as_deref() != Some(avatar_key)
                        })
                )
            {
                return Err(expected("error-stale-avatar-upload"));
            }

            let UpdateUser::MarkAvatarUploaded { repl } = oper else {
                unreachable!();
            };

            user_info.is_avatar_uploaded = Some(repl.is_avatar_uploaded);
        }

        None => user_info.last_active_at = now(),
    }

    user_info.updated_at = now();

    // Internal state field `UpdateUser`.
    // Internal implementation detail.
    if let UpdateUser::PasswordHash { repl } = oper {
        //
        // Mutate matching credential hash for the same user id.
        let credential = state
            .credentials
            .iter_mut()
            .find(|credential| credential.user_id == repl.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        credential.password_hash = repl.password_hash.clone();
    }

    accept(())
}

impl<'a> Run<GetUserInfo<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve `GetUserInfo` against the locked mock state.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &GetUserInfo<'a>) -> BaseRest<UserInfo> {
        //
        // Lock state immutably for read-only user info lookup.
        let state = self.state.lock().unwrap();

        match oper {
            GetUserInfo::Id { id } => get_user_info(&state, id),
        }
    }
}

impl<'a> Run<GetUserCredential<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve credentials from mock state by qid.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'a>,
    ) -> BaseRest<UserCredential> {
        //
        // Lock state immutably for safe credential read.
        let state = self.state.lock().unwrap();

        match oper {
            GetUserCredential::Qid { qid } => get_user_credential(&state, qid),
        }
    }
}

impl<'a> Run<FindUserInfo<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve optional user info by qid in shared state.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &FindUserInfo<'a>) -> BaseRest<Option<UserInfo>> {
        //
        // Lock state immutably for optional find-by-qid.
        let state = self.state.lock().unwrap();

        match oper {
            FindUserInfo::Qid { qid } => accept(find_user_info(&state, qid)),
        }
    }
}

impl<'a> Run<UpdateUser<'a>> for Mock {
    // Keep run layer errors as `BaseError` in tests.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply update user mutations under mutable lock.
    #[instrument(level = "info", skip_all)]
    async fn run(&self, oper: &UpdateUser<'a>) -> BaseRest<()> {
        //
        // Lock mutable state and reuse shared update helper.
        let mut state = self.state.lock().unwrap();

        update_user(&mut state, oper)
    }
}

impl<'a> Step<CreateUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Create user using transaction-local mutable state.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateUser<'a>,
    ) -> BaseRest<UserInfo> {
        create_user(&mut context.state, oper.entry)
    }
}

impl<'a> Step<FindUserInfo<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve optional user by qid from transaction context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindUserInfo<'a>,
    ) -> BaseRest<Option<UserInfo>> {
        //
        match oper {
            //
            FindUserInfo::Qid { qid } => {
                accept(find_user_info(&context.state, qid))
            }
        }
    }
}

impl<'a> Step<UpdateUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply update operation using context-scoped user state.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateUser<'a>,
    ) -> BaseRest<()> {
        update_user(&mut context.state, oper)
    }
}

impl<'a> Step<ReserveUserAvatar<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Reserve/reuse avatar metadata and return reservation detail.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReserveUserAvatar<'a>,
    ) -> BaseRest<UserAvatarReservation> {
        //
        // Locate user and branch on same-hash reuse or new hash allocation.
        let user_info = context
            .state
            .users
            .iter_mut()
            .find(|user_info| user_info.id == oper.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        let same_hash = user_info.avatar_key.is_some()
            && user_info.avatar_hash.as_ref() == Some(oper.image_hash);

        if same_hash && user_info.avatar_ext != Some(oper.image_ext) {
            return Err(expected("error-invalid-image-extension"));
        }

        if same_hash {
            //
            // Keep existing key when hash matches and extension is unchanged.
            let object_key = user_info.avatar_key.clone().ok_or_else(|| {
                //
                BaseError::Unrecoverable {
                    message: "[Mock::ReserveUserAvatar] avatar key is missing"
                        .into(),
                }
            })?;

            return accept(UserAvatarReservation {
                object_key,
                prev_object_key: None,
                avatar_version: user_info.avatar_version.ok_or_else(|| {
                    //
                    BaseError::Unrecoverable {
                        message: "[Mock::ReserveUserAvatar] avatar version is missing".into(),
                    }
                })?,
                is_upload_required: user_info.is_avatar_uploaded != Some(true),
            });
        }

        let avatar_version = user_info
            .avatar_version
            .unwrap_or(0)
            .checked_add(1)
            .ok_or_else(|| BaseError::Unrecoverable {
                message: "[Mock::ReserveUserAvatar] avatar version overflow"
                    .into(),
            })?;

        let object_key = UserComplex::gen_avatar_key(
            oper.id,
            avatar_version,
            oper.image_ext.suffix(),
        );

        let prev_object_key = user_info.avatar_key.clone();

        user_info.avatar_key = Some(object_key.clone());

        user_info.is_avatar_uploaded = Some(false);

        user_info.avatar_version = Some(avatar_version);

        user_info.avatar_hash = Some(oper.image_hash.clone());

        user_info.avatar_ext = Some(oper.image_ext);

        user_info.updated_at = now();

        accept(UserAvatarReservation {
            object_key,
            prev_object_key,
            avatar_version,
            is_upload_required: true,
        })
    }
}

impl<'a> Step<GetUserInfoExcluded<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Load by id with context state for exclusive update preparation.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetUserInfoExcluded<'a>,
    ) -> BaseRest<UserInfo> {
        //
        match oper {
            GetUserInfoExcluded::Id { id } => get_user_info(&context.state, id),
        }
    }
}

impl<'a> Step<DeleteUser<'a>, MockContext> for Mock {
    // Keep step errors as `BaseError` in mocked transactions.
    type Level = crate::part::nucl::RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Delete user and cleanup dependent mock artifacts.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteUser<'a>,
    ) -> BaseRest<()> {
        //
        // Remove user row and all related linked state for the same user id.
        let position = context
            .state
            .users
            .iter()
            .position(|user_info| user_info.id == oper.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        context.state.users.remove(position);

        context
            .state
            .credentials
            .retain(|credential| credential.user_id != oper.id);

        context
            .state
            .members
            .retain(|member_info| member_info.user_id != oper.id);

        context
            .state
            .member_invitations
            .retain(|info| info.invitor_id != oper.id);

        context
            .state
            .system_mails
            .retain(|mail| mail.receiver_id != oper.id);

        accept(())
    }
}
