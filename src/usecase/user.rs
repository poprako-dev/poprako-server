use futures_util::FutureExt as _;

use crate::domain::actor::user::hash_password;
use crate::domain::actor::user::sign_token;
use crate::domain::model::aggr::member::MemberForm;
use crate::domain::model::aggr::user::UserForm;
use crate::domain::model::aggr::user::UserToken;
use crate::domain::model::event::DomainEvent;
use crate::domain::model::event::EventSink;
use crate::domain::model::event::user::UserRegisteredEvent;
use crate::domain::query::QueryError;
use crate::domain::query::Transactional;
use crate::domain::query::member::MemberQueryMut;
use crate::domain::query::member_invitation::MemberInvitationQueryMut;
use crate::domain::query::user::UserQeuryMut;
use crate::domain::result::DomainError;
use crate::usecase::result::UseCaseError;
use crate::usecase::result::UseCaseRetVal;
use crate::usecase::val::user::{RegisterUserParams, RegisterUserRet};

#[tracing::instrument(skip(harn))]
pub async fn register_user<H>(
    harn: &H,
    params: RegisterUserParams,
) -> UseCaseRetVal<RegisterUserRet>
where
    H: Clone + Transactional,
{
    // Run the core registration logic inside a database transaction.
    let user_id: String = harn
        .run_in_transaction(move |query| {
            async move {
                // 1. Fetch pending invitation by invitee qid.
                let invitation = query
                    .get_pending_by_invitee_qid(&params.qid)
                    .await
                    .map_err(|e| match e {
                        QueryError::NotFound => DomainError::Expected("无效的邀请码".to_string()),
                        other => DomainError::from(other),
                    })?;

                // 2. Validate the invitation code.
                if !invitation.verify_code(&params.invitation_code) {
                    return Err(DomainError::Expected("无效的邀请码".to_string()));
                }

                // 3. Generate password hash.
                let password_hash = hash_password(&params.password).map_err(DomainError::from)?;

                // 4. Build the UserForm aggregate.
                let mut user_form =
                    UserForm::new(params.qid.clone(), params.nickname.clone(), password_hash);

                // Push domain event (publish happens after commit).
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
        .await
        .map_err(UseCaseError::from)?;

    // 7. Generate a signed token for the newly registered user.
    let token = sign_token(&UserToken::new(user_id.clone()))?;

    Ok(RegisterUserRet { user_id, token })
}
