use async_trait::async_trait;
use poprako_transactional::advance::Advance;

use crate::model::user::{UserAvatarReservation, UserCredential, UserForm, UserInfo};
use crate::part::query::Execute;
use crate::part::query::step::user::{
    Create, Delete, GetCredentialByQid, GetInfoById, GetInfoExcluded, MarkAvatarUploaded,
    ReserveAvatar, TouchLastActive, UpdateInfo,
};
use crate::part::query::user::{UserQuery, UserQueryTransactional};
use crate::part_impl::query_mock::{
    Mock, MockContext, MockState, MockTransactional, expected, now,
};
use crate::result::RootError;

impl UserQuery<MockContext> for Mock {}

impl UserQueryTransactional<MockContext> for MockTransactional {}

fn user_avatar_key(id: &str, avatar_version: i64, file_ext: &str) -> String {
    format!("user_avatar/{}-{}.{}", id, avatar_version, file_ext)
}

fn create_user(state: &mut MockState, form: &UserForm) -> Result<UserInfo, RootError> {
    if state.users.iter().any(|user| user.id == form.id) {
        return Err(expected("error-already-exists"));
    }
    if state.users.iter().any(|user| user.qid == form.qid) {
        return Err(expected("error-already-exists"));
    }

    let time = now();
    let user = UserInfo {
        id: form.id.clone(),
        qid: form.qid.clone(),
        nickname: form.nickname.clone(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin: false,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    };
    state.users.push(user.clone());
    state.credentials.push(UserCredential {
        user_id: form.id.clone(),
        password_hash: form.password_hash.clone(),
    });
    Ok(user)
}

#[async_trait]
impl<'a> Execute<GetInfoById<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetInfoById<'a>) -> Result<UserInfo, Self::Error> {
        let state = self.state.lock().unwrap();
        state
            .users
            .iter()
            .find(|user| user.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-user-not-found"))
    }
}

#[async_trait]
impl<'a> Execute<GetCredentialByQid<'a>> for Mock {
    type Error = RootError;

    async fn execute(&self, step: &GetCredentialByQid<'a>) -> Result<UserCredential, Self::Error> {
        let state = self.state.lock().unwrap();
        let user = state
            .users
            .iter()
            .find(|user| user.qid == step.qid)
            .ok_or_else(|| expected("error-user-not-found"));
        user.and_then(|user| {
            state
                .credentials
                .iter()
                .find(|credential| credential.user_id == user.id)
                .cloned()
                .ok_or_else(|| expected("error-user-not-found"))
        })
    }
}

#[async_trait]
impl<'a> Advance<Create<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Create<'a>,
    ) -> Result<UserInfo, Self::Error> {
        create_user(&mut context.state, step.form)
    }
}

#[async_trait]
impl<'a> Advance<UpdateInfo<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &UpdateInfo<'a>,
    ) -> Result<(), Self::Error> {
        let user = context
            .state
            .users
            .iter_mut()
            .find(|user| user.id == step.id)
            .ok_or_else(|| expected("error-user-not-found"))?;
        user.qid = step.qid.to_string();
        user.nickname = step.nickname.to_string();
        user.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<ReserveAvatar<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &ReserveAvatar<'a>,
    ) -> Result<UserAvatarReservation, Self::Error> {
        let user = context
            .state
            .users
            .iter_mut()
            .find(|user| user.id == step.id)
            .ok_or_else(|| expected("error-user-not-found"))?;
        let avatar_version = user.avatar_version + 1;
        let object_key = user_avatar_key(step.id, avatar_version, step.file_ext);
        let previous_object_key = user.avatar_key.clone();
        user.avatar_key = Some(object_key.clone());
        user.avatar_uploaded = false;
        user.avatar_version = avatar_version;
        user.updated_at = now();
        Ok(UserAvatarReservation {
            object_key,
            previous_object_key,
            avatar_version,
        })
    }
}

#[async_trait]
impl<'a> Advance<MarkAvatarUploaded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &MarkAvatarUploaded<'a>,
    ) -> Result<(), Self::Error> {
        let user = context
            .state
            .users
            .iter_mut()
            .find(|user| user.id == step.id)
            .ok_or_else(|| expected("error-user-not-found"))?;
        if user.avatar_version != step.avatar_version {
            return Err(expected("error-stale-avatar-upload"));
        }
        user.avatar_uploaded = true;
        user.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<TouchLastActive<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &TouchLastActive<'a>,
    ) -> Result<(), Self::Error> {
        let user = context
            .state
            .users
            .iter_mut()
            .find(|user| user.id == step.id)
            .ok_or_else(|| expected("error-user-not-found"))?;
        user.last_active_at = now();
        user.updated_at = now();
        Ok(())
    }
}

#[async_trait]
impl<'a> Advance<GetInfoExcluded<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &GetInfoExcluded<'a>,
    ) -> Result<UserInfo, Self::Error> {
        context
            .state
            .users
            .iter()
            .find(|user| user.id == step.id)
            .cloned()
            .ok_or_else(|| expected("error-user-not-found"))
    }
}

#[async_trait]
impl<'a> Advance<Delete<'a>, MockContext> for MockTransactional {
    type Error = RootError;

    async fn advance(
        &self,
        context: &mut MockContext,
        step: &Delete<'a>,
    ) -> Result<(), Self::Error> {
        let pos = context
            .state
            .users
            .iter()
            .position(|user| user.id == step.id)
            .ok_or_else(|| expected("error-user-not-found"))?;
        context.state.users.remove(pos);
        context
            .state
            .credentials
            .retain(|credential| credential.user_id != step.id);
        Ok(())
    }
}
