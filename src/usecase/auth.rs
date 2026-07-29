//! Authentication use cases — registration and login.

use poprako_orchestra::{Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_util::i18n::trl;

use crate::complex::member::MemberComplex;
use crate::complex::user::UserComplex;
use crate::data::auth::{
    LoginAuthParams, LoginAuthPayload, RegisterAuthParams, RegisterAuthPayload,
};
use crate::model::member::MemberEntry;
use crate::model::user::{UserEntry, UserToken};
use crate::part::auth::TokenAuth;
use crate::part::effect::EffectDevelop;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::member_invitation::MemberInvitationRepo;
use crate::part::repo::oper::member::CreateMember;
use crate::part::repo::oper::member_invitation::{
    GetMemberInvitationInfoExcluded, UpdateMemberInvitation,
};
use crate::part::repo::oper::user::{CreateUser, GetUserCredential};
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};

#[cfg(test)]
mod tests;

/// Registers a new user using an invitation code.
///
/// Within a single transaction, this function:
///
/// 1. Fetches and validates the invitation, ensuring the invitee QQ ID matches.
/// 2. Hashes the password via [`UserComplex::hash_password`].
/// 3. Inserts a new [`UserEntry`] row.
/// 4. Creates a [`MemberEntry`] linking the new user to the inviting team with
///    the role specified in the invitation.
/// 5. Marks the invitation as consumed.
///
/// After the transaction commits:
///
/// - A [`UserSignedUp`] event is emitted for side-effect processing.
/// - An authentication token is signed and returned.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Coordinates the transaction lifecycle.
/// * `C` — Context anchor (see the [repo module](crate::part::repo) for details).
/// * `R` — Repository bundle: [`UserRepo`], [`MemberRepo`], [`MemberInvitationRepo`].
/// * `A: TokenAuth` — Signs the session token.
/// * `V: EffectDevelop` — Processes the signup event.
#[instrument(
    level = "info",
    err(Debug),
    skip(nucl, repo, auth, develop, params),
    fields(
        qid = %params.qid,
        nickname = %params.nickname,
        password = "[REDACTED]",
        code = "[REDACTED]",
    ),
)]
pub async fn register<N, C, R, A, V>(
    (nucl, repo, auth, develop): (&N, &R, &A, &V),
    params: RegisterAuthParams,
) -> BaseRest<RegisterAuthPayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + MemberInvitationRepo<C> + Send + Sync,
    A: TokenAuth,
    V: EffectDevelop + Send + Sync,
{
    let (user_id, team_id, invitor_id, invitee_qid) = nucl
        .coord(async move |context| {
            //

            let invitation_info =
                GetMemberInvitationInfoExcluded::Code { code: &params.code }
                    .step_on(repo, context)
                    .await?;

            // Verify the invitation was issued for this QQ ID.
            if invitation_info.invitee_qid != params.qid {
                return Err(BaseError::Expected {
                    variant: ExpectedVariant::Args,
                    message: trl("error-invalid-invitation-code"),
                });
            }

            let password_hash =
                UserComplex::hash_password(&params.password).await?;

            let user_entry = UserEntry {
                id: UserComplex::gen_id(),
                qid: params.qid.clone(),
                nickname: params.nickname.clone(),
                password_hash,
            };

            let user_info = CreateUser { entry: &user_entry }
                .step_on(repo, context)
                .await?;

            let member_entry = MemberEntry {
                id: MemberComplex::gen_id(),
                user_id: user_info.id.clone(),
                user_nickname: user_info.nickname.clone(),
                team_id: invitation_info.team_id.clone(),
                roles: invitation_info.roles,
            };

            CreateMember {
                entry: &member_entry,
            }
            .step_on(repo, context)
            .await?;

            UpdateMemberInvitation::MarkUsed {
                id: &invitation_info.id,
            }
            .step_on(repo, context)
            .await?;

            accept((
                user_info.id,
                invitation_info.team_id,
                invitation_info.invitor_id,
                invitation_info.invitee_qid,
            ))
        })
        .await?;

    // Dispatch after successful commit so side effects do not run inside the transaction.
    let event = Event::UserSignedUp(UserSignedUpPayload {
        team_id: team_id.clone(),
        invitor_id,
        invitee_qid,
    });

    develop.develop(event).await;

    let original_token = UserToken { user_id };

    let token = auth.sign_token(&original_token)?;

    accept(RegisterAuthPayload {
        user_id: original_token.user_id,
        token,
    })
}

/// Authenticates a user with QQ ID and password.
///
/// This is a non-transactional read-only oper:
///
/// 1. Fetches the stored credential by QQ ID.
/// 2. Verifies the supplied password against the stored hash.
/// 3. Signs and returns an authentication token on success.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: UserRepo<C>` — Provides credential lookup.
/// * `A: TokenAuth` — Signs the session token.
#[instrument(
    level = "info",
    err(Debug),
    skip(repo, auth, params),
    fields(qid = %params.qid, password = "[REDACTED]"),
)]
pub async fn login<C, R, A>(
    (repo, auth): (&R, &A),
    params: LoginAuthParams,
) -> BaseRest<LoginAuthPayload>
where
    R: UserRepo<C>,
    A: TokenAuth,
{
    let user_credential = GetUserCredential::Qid { qid: &params.qid }
        .run_on(repo)
        .await?;

    if !UserComplex::verify_password(
        &params.password,
        &user_credential.password_hash,
    )
    .await
    {
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Auth,
            message: trl("error-wrong-credentials"),
        });
    }

    let original_token = UserToken {
        user_id: user_credential.user_id,
    };

    let token = auth.sign_token(&original_token)?;

    accept(LoginAuthPayload {
        user_id: original_token.user_id,
        token,
    })
}
