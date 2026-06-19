use uuid::Uuid;

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::atom::auth::hash_password;
use crate::data::auth::{LoginData, LoginVal, RegisterData, RegisterVal};
use crate::model::member::MemberForm;
use crate::model::user::{UserForm, UserToken};
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part::effect::{Develop, EffectEmit as _};
use crate::part::query::member::{MemberQuery, MemberQueryTransactional};
use crate::part::query::member_invitation::{
    MemberInvitationQuery, MemberInvitationQueryTransactional,
};
use crate::part::query::step::member::MemberStep;
use crate::part::query::step::member_invitation::MemberInvitationStep;
use crate::part::query::step::user::UserStep;
use crate::part::query::user::{UserQuery, UserQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::part::token::TokenAuth;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn register<D, H, Q, A, Dv>(
    drive: D,
    query: Q,
    auth: &A,
    develop: &Dv,
    data: RegisterData,
) -> RootResult<RegisterVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + MemberInvitationQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H>
        + MemberQueryTransactional<H>
        + MemberInvitationQueryTransactional<H>
        + Send,
    A: TokenAuth,
    Dv: Develop + Send + Sync,
{
    let (user_id, team_id, invitor_id, invitee_qid) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let invitation_info = query
                .advance(
                    handle,
                    MemberInvitationStep::get_info_by_code_excluded(&data.invitation_code),
                )
                .await?;

            if invitation_info.invitee_qid != data.qid {
                return Err(RootError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-invitation-code"),
                });
            }

            let password_hash = hash_password(&data.password)?;

            let user_form = UserForm {
                id: format!("user-{}", Uuid::now_v7()),
                qid: data.qid.clone(),
                nickname: data.nickname.clone(),
                password_hash,
            };

            let user_info = query.advance(handle, UserStep::create(&user_form)).await?;

            let member_form = MemberForm {
                id: format!("member-{}", Uuid::now_v7()),
                user_id: user_info.id.clone(),
                user_nickname: user_info.nickname.clone(),
                team_id: invitation_info.team_id.clone(),
                role_mask: invitation_info.role_mask,
            };

            query
                .advance(handle, MemberStep::create(&member_form))
                .await?;

            query
                .advance(
                    handle,
                    MemberInvitationStep::mark_pending_as_used(&invitation_info.id),
                )
                .await?;

            accept((
                user_info.id,
                invitation_info.team_id,
                invitation_info.invitor_id,
                invitation_info.invitee_qid,
            ))
        })
        .await
        .map_err(map_drive_err)?;

    Event::UserSignedUp(UserSignedUpPayload {
        team_id: team_id.clone(),
        invitor_id,
        invitee_qid,
    })
    .emit(develop)
    .await;

    let token = auth.sign(&UserToken {
        user_id: user_id.clone(),
    })?;

    Ok(RegisterVal { user_id, token })
}

pub async fn login<H, Q, A>(query: Q, auth: &A, data: LoginData) -> RootResult<LoginVal>
where
    Q: UserQuery<H>,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H>,
    A: TokenAuth,
{
    let credential = query
        .execute(UserStep::get_credential_by_qid(&data.qid))
        .await?;

    if !credential.verify_password(&data.password) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Auth,
            message: trl("error-wrong-credentials"),
        });
    }

    let token = auth.sign(&UserToken {
        user_id: credential.user_id.clone(),
    })?;

    Ok(LoginVal {
        user_id: credential.user_id,
        token,
    })
}
