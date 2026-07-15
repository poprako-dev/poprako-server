//! Authentication use cases — registration and login.

use tracing::instrument;

use poprako_orchestra::Nucl;

use poprako_util::i18n::trl;

use crate::complex::member::MemberComplex;
use crate::complex::user::UserComplex;
use crate::data::auth::{
    LoginAuthParams, LoginAuthPayload, RegisterAuthParams, RegisterAuthPayload,
};
use crate::model::member::MemberEntry;
use crate::model::user::{UserEntry, UserTokenRef};
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
use crate::result::{ExpectedVariant, RegularError, RegularResult};

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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn register<N, C, R, A, V>(
    nucl: &N,
    repo: &R,
    auth: &A,
    develop: &V,
    params: RegisterAuthParams,
) -> RegularResult<RegisterAuthPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + MemberInvitationRepo<C> + Send + Sync,
    A: TokenAuth,
    V: EffectDevelop + Send + Sync,
{
    let (user_id, team_id, invitor_id, invitee_qid) =
        nucl
            .coord(
                async move |context| -> RegularResult<(
                    String,
                    String,
                    String,
                    String,
                )> {
                    //

                    let invitation_info = repo
                        .step(
                            context,
                            &GetMemberInvitationInfoExcluded::Code {
                                code: &params.code,
                            },
                        )
                        .await?;

                    // Verify the invitation was issued for this QQ ID.
                    if invitation_info.invitee_qid != params.qid {
                        return Err(RegularError::Expected {
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

                    let user_info = repo
                        .step(context, &CreateUser { entry: &user_entry })
                        .await?;

                    let member_entry = MemberEntry {
                        id: MemberComplex::gen_id(),
                        user_id: user_info.id.clone(),
                        user_nickname: user_info.nickname.clone(),
                        team_id: invitation_info.team_id.clone(),
                        roles: invitation_info.roles,
                    };

                    repo.step(
                        context,
                        &CreateMember {
                            entry: &member_entry,
                        },
                    )
                    .await?;

                    repo.step(
                        context,
                        &UpdateMemberInvitation::MarkUsed {
                            id: &invitation_info.id,
                        },
                    )
                    .await?;

                    Ok((
                        user_info.id,
                        invitation_info.team_id,
                        invitation_info.invitor_id,
                        invitation_info.invitee_qid,
                    ))
                },
            )
            .await?;

    // Dispatch after successful commit so side effects do not run inside the transaction.
    let event = Event::UserSignedUp(UserSignedUpPayload {
        team_id: team_id.clone(),
        invitor_id,
        invitee_qid,
    });

    develop.develop(event).await;

    let token = auth.sign_token(&UserTokenRef { user_id: &user_id })?;

    Ok(RegisterAuthPayload { user_id, token })
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
#[instrument(level = "info", err(Debug), skip_all)]
pub async fn login<C, R, A>(
    repo: &R,
    auth: &A,
    params: LoginAuthParams,
) -> RegularResult<LoginAuthPayload>
where
    R: UserRepo<C>,
    A: TokenAuth,
{
    let user_credential = repo
        .run(&GetUserCredential::Qid { qid: &params.qid })
        .await?;

    if !UserComplex::verify_password(
        &params.password,
        &user_credential.password_hash,
    )
    .await
    {
        return Err(RegularError::Expected {
            variant: ExpectedVariant::Auth,
            message: trl("error-wrong-credentials"),
        });
    }

    let token = auth.sign_token(&UserTokenRef {
        user_id: &user_credential.user_id,
    })?;

    Ok(LoginAuthPayload {
        user_id: user_credential.user_id,
        token,
    })
}
