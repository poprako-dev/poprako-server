use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;
use uuid::Uuid;

use crate::atom;
use crate::data::auth::{LoginData, LoginVal, RegisterData, RegisterVal};
use crate::model::member::MemberForm;
use crate::model::user::{UserForm, UserToken};
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part::effect::{Develop, EffectEmit as _};
use crate::part::query::member::{MemberQuery, MemberQueryTransactional};
use crate::part::query::member_invitation::{
    MemberInvitationQuery,
    MemberInvitationQueryTransactional,
};
use crate::part::query::step::member::MemberCreate;
use crate::part::query::step::member_invitation::{
    MemberInvitationGetByCodeExcluded,
    MemberInvitationMarkPendingAsUsed,
};
use crate::part::query::step::user::{UserCreate, UserGetCredentialByQid};
use crate::part::query::user::{UserQuery, UserQueryTransactional};
use crate::part::query::{DeriveTransactional, Execute, map_drive_err};
use crate::part::token::TokenIssuer;
use crate::result::{ExpectedVariant, RootError, RootResult, accept};

pub async fn register<D, H, Q, T, E>(
    drive: D,
    query: Q,
    token_issuer: &T,
    develop: &E,
    input: RegisterData,
) -> RootResult<RegisterVal>
where
    D: Drive<H>,
    D::Error: Into<RootError>,
    H: Send,
    Q: UserQuery<H> + MemberQuery<H> + MemberInvitationQuery<H> + Send,
    <Q as DeriveTransactional>::Transactional:
        UserQueryTransactional<H>
        + MemberQueryTransactional<H>
        + MemberInvitationQueryTransactional<H>
        + Send,
    T: TokenIssuer,
    E: Develop + Send + Sync,
{
    let (user_id, team_id, invitor_id, invitee_qid) = drive
        .run_transactional(async move |handle| {
            let mut query = DeriveTransactional::transactional(&query).await;

            let invitation = query
                .advance(
                    handle,
                    MemberInvitationGetByCodeExcluded {
                        invitation_code: &input.invitation_code,
                    },
                )
                .await?;

            if invitation.invitee_qid != input.qid {
                return Err(RootError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-invitation-code"),
                });
            }

            let password_hash = atom::auth::hash_password(&input.password)?;

            let user_form = UserForm {
                id: format!("user-{}", Uuid::now_v7()),
                qid: input.qid.clone(),
                nickname: input.nickname.clone(),
                password_hash,
            };

            let user_info = query
                .advance(handle, UserCreate { form: &user_form })
                .await?;

            let member_form = MemberForm {
                id: format!("member-{}", Uuid::now_v7()),
                user_id: user_info.id.clone(),
                user_nickname: user_info.nickname.clone(),
                team_id: invitation.team_id.clone(),
                role_mask: invitation.role_mask,
            };

            query
                .advance(handle, MemberCreate { form: &member_form })
                .await?;

            query
                .advance(
                    handle,
                    MemberInvitationMarkPendingAsUsed {
                        id: &invitation.id,
                    },
                )
                .await?;

            accept((
                user_info.id,
                invitation.team_id,
                invitation.invitor_id,
                invitation.invitee_qid,
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

    let token = token_issuer.sign(&UserToken {
        user_id: user_id.clone(),
    })?;

    Ok(RegisterVal { user_id, token })
}

pub async fn login<H, Q, T>(
    query: Q,
    token_issuer: &T,
    input: LoginData,
) -> RootResult<LoginVal>
where
    Q: UserQuery<H>,
    <Q as DeriveTransactional>::Transactional: UserQueryTransactional<H>,
    T: TokenIssuer,
{
    let credential = Execute::execute(&query, UserGetCredentialByQid { qid: &input.qid }).await?;

    if !credential.verify_password(&input.password) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::Auth,
            message: trl("error-wrong-credentials"),
        });
    }

    let token = token_issuer.sign(&UserToken {
        user_id: credential.user_id.clone(),
    })?;

    Ok(LoginVal {
        user_id: credential.user_id,
        token,
    })
}
