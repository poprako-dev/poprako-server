use futures_util::FutureExt as _;
use tracing::instrument;

use crate::domain::actor::user::{hash_password, sign_token};
use crate::domain::model::aggregate::member::MemberForm;
use crate::domain::model::aggregate::user::{UserForm, UserToken};
use crate::domain::model::event::{DomainEvent, EventSink, user::UserRegisteredEvent};
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQueryMut;
use crate::domain::query::member_invitation::MemberInvitationQueryMut;
use crate::domain::query::user::UserQeuryMut;
use crate::domain::result::DomainErr;
use crate::usecase::result::UseCaseResl;
use crate::usecase::value_object::user::{SignUpUserParams, SignUpUserReply};
use crate::util::err::ErrorTrace as _;
use crate::util::i18n::trl;

#[instrument(skip(harn))]
pub async fn sign_up_user<H>(harn: &H, params: SignUpUserParams) -> UseCaseResl<SignUpUserReply>
where
    H: Clone + Transactional,
{
    // Run the core registration logic inside a database transaction.
    let user_id: String = harn
        .run_in_transaction(move |query| {
            async move {
                // 1. Fetch pending invitation by invitee qid.
                let invitation = query.get_pending_by_invitee_qid(&params.qid).await?;

                // 2. Validate the invitation code.
                if !invitation.verify_code(&params.invitation_code) {
                    return Err(DomainErr::expected_argument(trl(
                        "error-invalid-invitation-code",
                    )))
                    .trace_debug();
                }

                // 3. Generate password hash.
                let password_hash = hash_password(&params.password)?;

                // 4. Build the UserForm aggregate.
                let mut user_form =
                    UserForm::new(params.qid.clone(), params.nickname.clone(), password_hash);

                // Push domain event (publish happens after commit).
                // TODO: handle this event.
                user_form.push_event(DomainEvent::UserRegistered(UserRegisteredEvent {
                    team_id: invitation.team_id.clone(),
                    invitor_id: invitation.invitor_id.clone(),
                    invitor_qid: invitation.invitee_qid.clone(),
                }));

                // 5. Create the user.
                let user = UserQeuryMut::create(query, user_form).await?;

                // 6. Create a member record so the user belongs to the team.
                let member_form = MemberForm::new(
                    user.id.clone(),
                    user.nickname.clone(),
                    invitation.team_id.clone(),
                    invitation.roles,
                );

                MemberQueryMut::create(query, member_form).await?;

                // 7. Mark the invitation as consumed.
                query.mark_as_used(&invitation.id).await?;

                Ok(user.id.clone())
            }
            .boxed()
        })
        .await?;

    // 7. Generate a signed token for the newly registered user.
    let token = sign_token(&UserToken::new(user_id.clone()))?;

    Ok(SignUpUserReply { user_id, token })
}
