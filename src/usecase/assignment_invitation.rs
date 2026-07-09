//! Assignment invitation use cases.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;
use poprako_util::i18n::trl;
use poprako_util::page::Page;

use crate::complex::assignment::{AssignmentComplex, AssignmentPermComplex};
use crate::data::assignment::AssignmentInfoVal;
use crate::data::assignment_invitation::{
    AssignmentInvitationInfoVal, CreateAssignmentInvitationData,
    CreateAssignmentInvitationVal, JoinAssignmentInvitationData,
    ListAssignmentInvitationInfosData,
};
use crate::model::assignment::AssignmentForm;
use crate::model::assignment_invitation::AssignmentInvitationForm;
use crate::model::user::UserToken;
use crate::part::image::ImagePool;
use crate::part::repo::assignment::{
    AssignmentRepo, AssignmentRepoTransactional,
};
use crate::part::repo::assignment_invitation::{
    AssignmentInvitationRepo, AssignmentInvitationRepoTransactional,
};
use crate::part::repo::chapter::{ChapterRepo, ChapterRepoTransactional};
use crate::part::repo::comic::{ComicRepo, ComicRepoTransactional};
use crate::part::repo::map_drive_err;
use crate::part::repo::member::{MemberRepo, MemberRepoTransactional};
use crate::part::repo::step::assignment::AssignmentStep;
use crate::part::repo::step::assignment_invitation::AssignmentInvitationStep;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::member::MemberStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::repo::user::{UserRepo, UserRepoTransactional};
use crate::part::repo::workset::{WorksetRepo, WorksetRepoTransactional};
use crate::result::{ExpectedVariant, RegularError, RegularResult, accept};
use crate::util::{DeriveTransactional, next_snowflake_id};
use crate::value::role::{RoleField, RoleMask};

#[cfg(test)]
mod tests;

// FIXME: invitations should be fired out after a period of time.

/// Lists assignment invitations under one chapter.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    data: ListAssignmentInvitationInfosData,
) -> RegularResult<Vec<AssignmentInvitationInfoVal>>
where
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Sync,
    <R as DeriveTransactional>::Transactional:
        AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>,
{
    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_admin(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.chapter_id,
    )
    .await?;

    let assignment_invitation_infos = repo
        .execute(&AssignmentInvitationStep::list_infos(
            &data.chapter_id,
            data.pending,
            Page {
                offset: data.offset,
                limit: data.limit,
            },
        ))
        .await?;

    accept(
        assignment_invitation_infos
            .into_iter()
            .map(AssignmentInvitationInfoVal::from)
            .collect(),
    )
}

/// Creates a pending assignment invitation.
pub async fn create<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    data: CreateAssignmentInvitationData,
) -> RegularResult<CreateAssignmentInvitationVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UserRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UserRepoTransactional<C>
            + Send
            + Sync,
{
    validate_roles(data.roles)?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_admin(
        &mut repo.as_proxy(),
        &token.user_id,
        &data.chapter_id,
    )
    .await?;

    let (assignment_invitation_id, code) = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let invitee_user_info = repo
                .advance(
                    context,
                    &UserStep::find_info_by_qid(&data.invitee_qid),
                )
                .await?;

            if let Some(invitee_user_info) = invitee_user_info {
                let existing_assignment_info = repo
                    .advance(
                        context,
                        &AssignmentStep::get_info_by_chapter_id_and_user_id(
                            &data.chapter_id,
                            &invitee_user_info.id,
                        ),
                    )
                    .await?;

                if existing_assignment_info.is_some() {
                    return Err(invitee_assigned_error());
                }
            }

            let assignment_invitation_id = gen_assignment_invitation_id();
            let code = gen_code();

            let assignment_invitation_form = AssignmentInvitationForm {
                id: assignment_invitation_id,
                chapter_id: data.chapter_id,
                inviter_id: token.user_id,
                invitee_qid: data.invitee_qid,
                code,
                roles: data.roles,
            };

            let assignment_invitation_info = repo
                .advance(
                    context,
                    &AssignmentInvitationStep::create(
                        &assignment_invitation_form,
                    ),
                )
                .await?;

            accept((
                assignment_invitation_info.id,
                assignment_invitation_info.code,
            ))
        })
        .await
        .map_err(map_drive_err)?;

    accept(CreateAssignmentInvitationVal {
        id: assignment_invitation_id,
        code,
    })
}

/// Deletes an assignment invitation.
pub async fn delete<D, C, R>(
    drive: &D,
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentInvitationRepo<C> + AssignmentRepo<C> + Send + Sync,
    <R as DeriveTransactional>::Transactional:
        AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + Send
            + Sync,
{
    let assignment_invitation_info = repo
        .execute(&AssignmentInvitationStep::get_info_by_id(&id))
        .await?;

    use crate::part::shared::proxy::AsProxyNonTransactional as _;

    AssignmentPermComplex::can_user_admin(
        &mut repo.as_proxy(),
        &token.user_id,
        &assignment_invitation_info.chapter_id,
    )
    .await?;

    drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            repo.advance(context, &AssignmentInvitationStep::delete(&id))
                .await?;

            accept(())
        })
        .await
        .map_err(map_drive_err)?;

    accept(())
}

/// Joins a chapter assignment with a pending invitation code.
pub async fn join<D, C, R, I>(
    drive: &D,
    repo: &R,
    image_pool: &I,
    token: UserToken,
    data: JoinAssignmentInvitationData,
) -> RegularResult<AssignmentInfoVal>
where
    D: Drive<C>,
    D::Error: Into<RegularError>,
    C: Send,
    R: AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UserRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + Send
        + Sync,
    <R as DeriveTransactional>::Transactional:
        AssignmentInvitationRepoTransactional<C>
            + AssignmentRepoTransactional<C>
            + UserRepoTransactional<C>
            + MemberRepoTransactional<C>
            + ChapterRepoTransactional<C>
            + ComicRepoTransactional<C>
            + WorksetRepoTransactional<C>
            + Send
            + Sync,
    I: ImagePool,
{
    let current_user_id = token.user_id;

    let assignment_info = drive
        .with_context(async move |context| {
            let repo = repo.derive_transactional().await;

            let current_user_info = repo
                .advance(
                    context,
                    &UserStep::get_info_excluded(&current_user_id),
                )
                .await?;

            let assignment_invitation_info = repo
                .advance(
                    context,
                    &AssignmentInvitationStep::get_info_by_code_excluded(
                        &data.code,
                    ),
                )
                .await?;

            if assignment_invitation_info.invitee_qid != current_user_info.qid {
                return Err(invalid_invitation_error());
            }

            validate_roles(assignment_invitation_info.roles)?;

            let chapter_info = repo
                .advance(
                    context,
                    &ChapterStep::get_info_by_id(
                        &assignment_invitation_info.chapter_id,
                        &[],
                    ),
                )
                .await?;

            let comic_info = repo
                .advance(
                    context,
                    &ComicStep::get_info_by_id(&chapter_info.comic_id, &[]),
                )
                .await?;

            let workset_info = repo
                .advance(
                    context,
                    &WorksetStep::get_info_by_id(&comic_info.workset_id),
                )
                .await?;

            let member_info = repo
                .advance(
                    context,
                    &MemberStep::find_info_by_user_id_and_team_id(
                        &current_user_id,
                        &workset_info.team_id,
                    ),
                )
                .await?;

            let Some(member_info) = member_info else {
                return Err(assignment_role_not_assignable_perm_error());
            };

            if !member_info
                .roles
                .contains_mask(assignment_invitation_info.roles)
            {
                return Err(assignment_role_not_assignable_perm_error());
            }

            let existing_assignment_info = repo
                .advance(
                    context,
                    &AssignmentStep::get_info_by_chapter_id_and_user_id(
                        &assignment_invitation_info.chapter_id,
                        &current_user_id,
                    ),
                )
                .await?;

            let assignment_info = match existing_assignment_info {
                Some(existing_assignment_info) => {
                    let assignment_role_update = AssignmentComplex::merge_roles(
                        &existing_assignment_info,
                        assignment_invitation_info.roles,
                    );
                    repo.advance(
                        context,
                        &AssignmentStep::put_roles(&assignment_role_update),
                    )
                    .await?
                }
                None => {
                    let assignment_form = AssignmentForm {
                        id: AssignmentComplex::gen_id(),
                        chapter_id: assignment_invitation_info
                            .chapter_id
                            .clone(),
                        user_id: current_user_id,
                        roles: assignment_invitation_info.roles,
                    };
                    repo.advance(
                        context,
                        &AssignmentStep::create(&assignment_form),
                    )
                    .await?
                }
            };

            repo.advance(
                context,
                &AssignmentInvitationStep::mark_pending_as_used(
                    &assignment_invitation_info.id,
                ),
            )
            .await?;

            accept(assignment_info)
        })
        .await
        .map_err(map_drive_err)?;

    AssignmentInfoVal::from_model(image_pool, assignment_info).await
}

fn gen_assignment_invitation_id() -> String {
    next_snowflake_id()
}

fn gen_code() -> String {
    let id = next_snowflake_id();
    let len = id.len();

    if len <= 6 {
        return id;
    }

    id[len - 6..].to_string()
}

fn validate_roles(roles: RoleMask) -> RegularResult<()> {
    if u32::from(roles) == 0 || roles.has_any_role(&[RoleField::ADMIN]) {
        return Err(assignment_role_not_assignable_args_error());
    }
    accept(())
}

fn invalid_invitation_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-no-pending-invitation"),
    }
}

fn invitee_assigned_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-assignment-already-exists"),
    }
}

fn assignment_role_not_assignable_args_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Args,
        message: trl("error-chapter-role-not-assignable"),
    }
}

fn assignment_role_not_assignable_perm_error() -> RegularError {
    RegularError::Expected {
        variant: ExpectedVariant::Perm,
        message: trl("error-chapter-role-not-assignable"),
    }
}
