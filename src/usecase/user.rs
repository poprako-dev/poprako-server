use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::compound::user::{hash_password, sign_token};
use crate::domain::effect::{Effect as _, EffectSink};
use crate::domain::external::token::TokenCodec;
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::aggregate::user::{UserForm, UserToken};
use crate::domain::model::event::{Event, EventSink, user::UserSignedUpEvent};
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQueryTransactional;
use crate::domain::query::member_invitation::MemberInvitationQueryTransactional;
use crate::domain::query::user::UserQueryTransactional;
use crate::domain::result::DomainError;
use crate::usecase::result::UseCaseResult;
use crate::usecase::value_object::user::{SignUpUserParams, SignUpUserReply};
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(harn))]
pub async fn sign_up_user<H>(harn: &H, params: SignUpUserParams) -> UseCaseResult<SignUpUserReply>
where
    H: Clone + Transactional + EffectSink + TokenCodec + Send + Sync,
{
    // Run the core registration logic inside a database transaction.
    let mut user_form = harn
        .run_in_transaction(move |query| {
            async move {
                // 1. Acquire an exclusive row lock on the pending invitation by its code.
                //    This serialises concurrent attempts to consume the same invitation.
                let invitation = MemberInvitationQueryTransactional::get_by_code_ex(
                    query,
                    params.invitation_code.clone(),
                )
                .await?;

                // 2. Validate the invitee identity.
                if invitation.invitee_qid != params.qid {
                    return Err(DomainError::expected_argument(trl(
                        "error-invalid-invitation-code",
                    )))
                    .trace_debug();
                }

                // 3. Generate password hash.
                let password_hash = hash_password(&params.password)?;

                // 4. Build the UserForm aggregate.
                let mut user_form =
                    UserForm::new(params.qid.clone(), params.nickname.clone(), password_hash);

                // Push domain event (published after commit via effect sink).
                user_form.push_event(Event::UserSignedUp(UserSignedUpEvent {
                    team_id: invitation.team_id.clone(),
                    invitor_id: invitation.invitor_id.clone(),
                    invitee_qid: params.qid.clone(),
                }));

                // 5. Create the user.
                let user =
                    UserQueryTransactional::create(query, user_form.clone_without_events()).await?;

                // 6. Create a member record so the user belongs to the team.
                let member_form = MemberForm::new(
                    user.id.clone(),
                    user.nickname.clone(),
                    invitation.team_id.clone(),
                    invitation.roles,
                );

                MemberQueryTransactional::create(query, member_form).await?;

                // 7. Mark the invitation as consumed.
                MemberInvitationQueryTransactional::mark_pending_as_used(query, invitation.id).await?;

                Ok(user_form)
            }
            .boxed()
        })
        .await?;

    // Publish events after successful transaction commit.
    user_form.run_effect(harn).await;

    // Generate a signed token for the newly registered user.
    let token = sign_token(harn, &UserToken::new(user_form.id.clone()))?;

    Ok(SignUpUserReply {
        user_id: user_form.id,
        token,
    })
}
