//! Mock user repository operations.

use poprako_orchestra::{Run, Step};

use tracing::instrument;

use crate::complex::user::UserComplex;
use crate::model::user::{
    UserAvatarReservation, UserCredential, UserEntry, UserInfo,
};
use crate::part::repo::oper::user::{
    CreateUser, DeleteUser, FindUserInfo, GetUserCredential, GetUserInfo,
    GetUserInfoExcluded, ReserveUserAvatar, UpdateUser,
};
use crate::part::repo::user::UserRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{RegularError, RegularResult};

impl UserRepo<MockContext> for Mock {}

fn create_user(
    state: &mut MockState,
    entry: &UserEntry,
) -> RegularResult<UserInfo> {
    //
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
        avatar_uploaded: false,
        avatar_version: 0,
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

    Ok(user_info)
}

fn get_user_info(state: &MockState, id: &str) -> RegularResult<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-user-not-found"))
}

fn find_user_info(state: &MockState, qid: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.qid == qid)
        .cloned()
}

fn get_user_credential(
    state: &MockState,
    qid: &str,
) -> RegularResult<UserCredential> {
    //
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

fn update_user(
    state: &mut MockState,
    oper: &UpdateUser<'_>,
) -> RegularResult<()> {
    //
    let (id, update) = match oper {
        //
        UpdateUser::Info { id, qid, nickname } => {
            (id, Some((qid, nickname, None)))
        }

        UpdateUser::MarkAvatarUploaded { id, avatar_version } => {
            (id, Some((id, id, Some(*avatar_version))))
        }

        UpdateUser::TouchLastActive { id } => (id, None),

        UpdateUser::PasswordHash { id, .. } => (id, None),
    };

    let user_info = state
        .users
        .iter_mut()
        .find(|user_info| user_info.id == *id)
        .ok_or_else(|| expected("error-user-not-found"))?;

    match update {
        //
        Some((qid, nickname, None)) => {
            //
            user_info.qid = qid.to_string();

            user_info.nickname = nickname.to_string();
        }

        Some((_, _, Some(avatar_version))) => {
            //
            if user_info.avatar_version != avatar_version {
                return Err(expected("error-stale-avatar-upload"));
            }

            user_info.avatar_uploaded = true;
        }

        None => user_info.last_active_at = now(),
    }

    user_info.updated_at = now();

    match oper {
        //
        UpdateUser::PasswordHash { id, password_hash } => {
            let credential = state
                .credentials
                .iter_mut()
                .find(|credential| credential.user_id == *id)
                .ok_or_else(|| expected("error-user-not-found"))?;

            credential.password_hash = password_hash.to_string();
        }

        UpdateUser::Info { .. }
        | UpdateUser::MarkAvatarUploaded { .. }
        | UpdateUser::TouchLastActive { .. } => {}
    }

    Ok(())
}

impl<'a> Run<GetUserInfo<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &GetUserInfo<'a>) -> RegularResult<UserInfo> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            GetUserInfo::Id { id } => get_user_info(&state, id),
        }
    }
}

impl<'a> Run<GetUserCredential<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetUserCredential<'a>,
    ) -> RegularResult<UserCredential> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            GetUserCredential::Qid { qid } => get_user_credential(&state, qid),
        }
    }
}

impl<'a> Run<FindUserInfo<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &FindUserInfo<'a>,
    ) -> RegularResult<Option<UserInfo>> {
        //
        let state = self.state.lock().unwrap();

        match oper {
            FindUserInfo::Qid { qid } => Ok(find_user_info(&state, qid)),
        }
    }
}

impl<'a> Run<UpdateUser<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateUser<'a>) -> RegularResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        update_user(&mut state, oper)
    }
}

impl<'a> Step<CreateUser<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateUser<'a>,
    ) -> RegularResult<UserInfo> {
        create_user(&mut context.state, oper.entry)
    }
}

impl<'a> Step<FindUserInfo<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &FindUserInfo<'a>,
    ) -> RegularResult<Option<UserInfo>> {
        match oper {
            FindUserInfo::Qid { qid } => {
                Ok(find_user_info(&context.state, qid))
            }
        }
    }
}

impl<'a> Step<UpdateUser<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateUser<'a>,
    ) -> RegularResult<()> {
        update_user(&mut context.state, oper)
    }
}

impl<'a> Step<ReserveUserAvatar<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ReserveUserAvatar<'a>,
    ) -> RegularResult<UserAvatarReservation> {
        //
        let user_info = context
            .state
            .users
            .iter_mut()
            .find(|user_info| user_info.id == oper.id)
            .ok_or_else(|| expected("error-user-not-found"))?;

        let avatar_version = user_info.avatar_version + 1;

        let object_key =
            UserComplex::gen_avatar_key(oper.id, avatar_version, oper.file_ext);

        let prev_object_key = user_info.avatar_key.clone();

        user_info.avatar_key = Some(object_key.clone());

        user_info.avatar_uploaded = false;

        user_info.avatar_version = avatar_version;

        user_info.updated_at = now();

        Ok(UserAvatarReservation {
            object_key,
            prev_object_key,
            avatar_version,
        })
    }
}

impl<'a> Step<GetUserInfoExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetUserInfoExcluded<'a>,
    ) -> RegularResult<UserInfo> {
        match oper {
            GetUserInfoExcluded::Id { id } => get_user_info(&context.state, id),
        }
    }
}

impl<'a> Step<DeleteUser<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteUser<'a>,
    ) -> RegularResult<()> {
        //
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

        Ok(())
    }
}
