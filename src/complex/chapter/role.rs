use poprako_orchestra::{OperProxy as _, Proxy};

use poprako_util::i18n::trl;

use crate::model::read::proj::chapter::ChapterInfo;
use crate::part::repo::oper::assignment::{
    FindAssignmentInfo, ListAssignmentInfos,
};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::value::chapter::{Stage, StageOper};
use crate::value::role::{RoleField, RoleMask};

/// Verify the caller holds the workflow role required for `oper` on `stage`.
///
/// | Stage | `Advance` | `Revert` |
/// |---|---|---|
/// | `RawProvide` | `RAW_PROVIDER` | - |
/// | `Translate` | `TRANSLATOR` | `PROOFREADER` |
/// | `Proofread` | `PROOFREADER` | `PROOFREADER` |
/// | `TypesetRedraw` | `TYPESETTER` or `REDRAWER` | `TYPESETTER` or `REDRAWER` |
/// | `Review` | `REVIEWER` | `REVIEWER` |
/// | `Publish` | `PUBLISHER` | - |
pub async fn check_workflow_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_id: &str,
    stage: Stage,
    oper: StageOper,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<FindAssignmentInfo<'a, 'b>, Error = BaseError>
        + for<'a, 'b> Proxy<ListAssignmentInfos<'a, 'b>, Error = BaseError>,
{
    let assignment_info = FindAssignmentInfo::ChapterUser {
        chapter_id,
        user_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(assignment_info) = assignment_info else {
        //
        let err_message = trl("error-chapter-workflow-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            stage = ?stage,
            oper = ?oper,
            "expected error: workflow assignment missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    // Domain invariant: a workflow stage cannot be advanced unless at least
    // one person on the chapter holds the required workflow role. This runs
    // before the admin bypass so that even admins must ensure the position is
    // filled.
    if oper == StageOper::Advance {
        check_chapter_has_role_holder(proxy, chapter_id, stage).await?;
    }

    let roles = assignment_info.roles;

    if roles.has_any_role(&[RoleField::ADMIN]) {
        return accept(());
    }

    let required_roles = required_roles_for_transition(stage, oper);

    if required_roles.is_empty() {
        //
        let err_message = trl("error-chapter-workflow-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            stage = ?stage,
            oper = ?oper,
            assignment_roles = ?roles,
            required_roles = ?required_roles,
            "expected error: workflow transition role is not configured",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    if !roles.has_any_role(required_roles) {
        //
        let err_message = trl("error-chapter-workflow-role-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_id,
            stage = ?stage,
            oper = ?oper,
            assignment_roles = ?roles,
            required_roles = ?required_roles,
            "expected error: workflow role missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

/// Verify the caller may join a chapter with the given role mask.
///
/// Rejects `ADMIN` roles (not assignable through the join flow). The caller
/// must be a team member whose membership [`RoleMask`] contains the requested
/// role bits.
pub async fn check_join_role<P>(
    proxy: &mut P,
    user_id: &str,
    chapter_info: &ChapterInfo,
    roles: RoleMask,
) -> BaseRest<()>
where
    P: for<'a> Proxy<ResolveTeamId<'a>, Error = BaseError>
        + for<'a> Proxy<FindMemberInfo<'a>, Error = BaseError>,
{
    if roles.has_any_role(&[RoleField::ADMIN]) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Args,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_info.id,
            comic_id = %chapter_info.comic_id,
            roles = ?roles,
            "expected error: admin role is not assignable through join",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Args,
            message: err_message,
        });
    }

    let team_id = ResolveTeamId::Comic {
        id: &chapter_info.comic_id,
    }
    .proxy_on(proxy)
    .await?;

    let member_info = FindMemberInfo::UserTeam {
        user_id,
        team_id: &team_id,
    }
    .proxy_on(proxy)
    .await?;

    let Some(member_info) = member_info else {
        //
        let err_message = trl("error-team-member-required");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_info.id,
            comic_id = %chapter_info.comic_id,
            team_id = %team_id,
            roles = ?roles,
            "expected error: chapter team member is missing",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    };

    if !member_info.roles.contains_mask(roles) {
        //
        let err_message = trl("error-chapter-role-not-assignable");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %user_id,
            chapter_id = %chapter_info.id,
            comic_id = %chapter_info.comic_id,
            team_id = %team_id,
            roles = ?roles,
            member_roles = ?member_info.roles,
            "expected error: chapter member lacks requested roles",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify that at least one person on the chapter holds the role(s) required
// for advancing `stage`. A workflow stage cannot be advanced unless someone
// is assigned to the corresponding role.
//
// Called only for [`StageOper::Advance`]; revert operations do not require a
// role holder.
async fn check_chapter_has_role_holder<P>(
    proxy: &mut P,
    chapter_id: &str,
    stage: Stage,
) -> BaseRest<()>
where
    P: for<'a, 'b> Proxy<ListAssignmentInfos<'a, 'b>, Error = BaseError>,
{
    let required_roles =
        required_roles_for_transition(stage, StageOper::Advance);

    let assignment_infos = ListAssignmentInfos::Chapter {
        chapter_id,
        role: None,
        incls: &[],
    }
    .proxy_on(proxy)
    .await?;

    let has_holder = assignment_infos
        .iter()
        .any(|info| info.roles.has_any_role(required_roles));

    if !has_holder {
        //
        let err_message = trl("error-chapter-no-role-holder");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            chapter_id = %chapter_id,
            stage = ?stage,
            required_roles = ?required_roles,
            "expected error: chapter workflow role has no holder",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    accept(())
}

// Verify the caller is permitted to perform the given workflow transition
// on the chapter. Chapter admins bypass per-stage checks and are allowed any
// transition. Other assignments are validated against a whitelist.
//
// Return the workflow roles required to perform `oper` on `stage`.
//
// Returns an empty slice when the combination is unlisted (i.e., disallowed
// unless the caller holds `ADMIN`).
fn required_roles_for_transition(
    stage: Stage,
    oper: StageOper,
) -> &'static [RoleField] {
    //
    match (stage, oper) {
        //
        (Stage::RawProvide, StageOper::Advance) => &[RoleField::RAW_PROVIDER],

        (Stage::Translate, StageOper::Advance) => &[RoleField::TRANSLATOR],

        (Stage::Translate, StageOper::Revert) => {
            &[RoleField::TRANSLATOR, RoleField::PROOFREADER]
        }

        (Stage::Proofread, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::PROOFREADER]
        }

        (Stage::TypesetRedraw, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::TYPESETTER, RoleField::REDRAWER]
        }

        (Stage::Review, StageOper::Advance | StageOper::Revert) => {
            &[RoleField::REVIEWER]
        }

        (Stage::Publish, StageOper::Advance) => &[RoleField::PUBLISHER],

        _ => &[],
    }
}
