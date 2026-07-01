//! Authentication use cases — registration and login.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;

use crate::complex::member::MemberComplex;
use crate::complex::user::UserComplex;
use crate::data::auth::{LoginData, LoginVal, RegisterData, RegisterVal};
use crate::model::member::MemberForm;
use crate::model::user::{UserForm, UserTokenRef};
use crate::part::auth::TokenAuth;
use crate::part::effect::event::Event;
use crate::part::effect::event::user::UserSignedUpPayload;
use crate::part::effect::{EffectDevelop, EffectEmit as _};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::member_invitation::{
    MemberInvitationRepo, MemberInvitationRepoTransactional,
};
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::member_invitation::MemberInvitationStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::result::{ExpectedVariant, RootError, RootResult, accept};
use crate::util::DeriveTransactional;

#[cfg(test)]
pub(crate) mod tests;

/// Registers a new user using an invitation code.
///
/// Within a single transaction, this function:
///
/// 1. Fetches and validates the invitation, ensuring the invitee QQ ID matches.
/// 2. Hashes the password via [`UserComplex::hash_password`].
/// 3. Inserts a new [`UserForm`] row.
/// 4. Creates a [`MemberForm`] linking the new user to the inviting team with
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
/// * `D: Drive<C>` — Drives the transaction lifecycle.
/// * `C` — Context anchor (see the [repo module](crate::part::repo) for details).
/// * `R` — Repository bundle: [`UserRepo`], [`MemberRepo`], [`MemberInvitationRepo`].
/// * `A: TokenAuth` — Signs the session token.
/// * `V: EffectDevelop` — Processes the signup event.
pub async fn register<D, C, R, A, V>(
    drive: &D,
    repo: &R,
    auth: &A,
    develop: &V,
    data: RegisterData,
) -> RootResult<RegisterVal>
where
    D: Drive<C>,
    D::Error: Into<RootError>,
    C: Send,
    R: UserRepo<C> + MemberRepo<C> + MemberInvitationRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C>
        + MemberRepoTransactional<C>
        + MemberInvitationRepoTransactional<C>
        + Send,
    A: TokenAuth,
    V: EffectDevelop + Send + Sync,
{
    let (user_id, team_id, invitor_id, invitee_qid) = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let invitation_info = repo
                .advance(
                    context,
                    &MemberInvitationStep::get_info_by_code_excluded(&data.code),
                )
                .await?;

            // Verify the invitation was issued for this QQ ID.
            if invitation_info.invitee_qid != data.qid {
                return Err(RootError::Expected {
                    variant: ExpectedVariant::ArgsInvalid,
                    message: trl("error-invalid-invitation-code"),
                });
            }

            let password_hash = UserComplex::hash_password(&data.password)?;

            let user_form = UserForm {
                id: UserComplex::gen_id(),
                qid: data.qid.clone(),
                nickname: data.nickname.clone(),
                password_hash,
            };

            let user_info = repo.advance(context, &UserStep::create(&user_form)).await?;

            let member_form = MemberForm {
                id: MemberComplex::gen_id(),
                user_id: user_info.id.clone(),
                user_nickname: user_info.nickname.clone(),
                team_id: invitation_info.team_id.clone(),
                roles: invitation_info.roles,
            };

            repo.advance(context, &MemberStep::create(&member_form))
                .await?;

            repo.advance(
                context,
                &MemberInvitationStep::mark_pending_as_used(&invitation_info.id),
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

    // Emit event after successful commit so side-effects don't run inside the transaction.
    Event::UserSignedUp(UserSignedUpPayload {
        team_id: team_id.clone(),
        invitor_id,
        invitee_qid,
    })
    .emit(develop)
    .await;

    let token = auth.sign_token(&UserTokenRef { user_id: &user_id })?;

    Ok(RegisterVal { user_id, token })
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
pub async fn login<C, R, A>(repo: &R, auth: &A, data: LoginData) -> RootResult<LoginVal>
where
    R: UserRepo<C>,
    <R as DeriveTransactional>::Transactional: UserRepoTransactional<C>,
    A: TokenAuth,
{
    let user_credential = repo
        .execute(&UserStep::get_credential_by_qid(&data.qid))
        .await?;

    if !UserComplex::verify_password(&data.password, &user_credential.password_hash) {
        return Err(RootError::Expected {
            variant: ExpectedVariant::AuthFail,
            message: trl("error-wrong-credentials"),
        });
    }

    let token = auth.sign_token(&UserTokenRef {
        user_id: &user_credential.user_id,
    })?;

    Ok(LoginVal {
        user_id: user_credential.user_id,
        token,
    })
}
