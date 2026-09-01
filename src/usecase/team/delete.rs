//! Team deletion orchestration.

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use poprako_obj_dept::ObjDept;
use poprako_obj_dept::oper::DeleteObjs;
use poprako_util::i18n::trl;

use crate::complex::team::TeamPermComplex;
use crate::model::shared::user::UserToken;
use crate::part::nucl::Serial;
use crate::part::obj_dept::{ComicCover, PageImage, TeamAvatar};
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::chapter_workflow_record::ChapterWorkflowRecordRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::comic_archive::ComicArchiveRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::{
    DeleteMember, FindMemberInfo, ListMemberInfosExcluded,
};
use crate::part::repo::oper::team::{DeleteTeam, GetTeamInfoExcluded};
use crate::part::repo::oper::workset::ListWorksetInfosExcluded;
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::termbase::delete_team_cascade;
use crate::usecase::workset::delete_cascade as delete_workset_cascade;

/// Deletes a team and all associated instr.
///
/// Transactional cascade:
///
/// 1. Fetches the team info with a pessimistic lock.
/// 2. Lists all worksets belonging to the team.
/// 3. Deletes descendant worksets and comics through their own delete paths.
/// 4. Enqueues avatar deletion if the team had an uploaded avatar.
/// 5. Deletes the team itself.
///
/// # Type Parameters
///
/// * `N: Nucl<Context = C>` — Coordination nucleus.
/// * `C` — Context anchor.
/// * `R: TeamRepo<C> + WorksetRepo<C> + ComicRepo<C>` — Team, workset, and comic storage.
/// * `P: Prom<C>` — Prom enqueuer for deferred avatar deletion.
#[instrument(level = "info", skip(nucl, repo, obj_dept, token), fields(actor_user_id = %token.user_id))]
pub async fn delete<N, C, R, O>(
    (nucl, repo, obj_dept): (&N, &R, &O),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError> + Sync,
    C::Level: AtLeast<Serial>,
    R: TeamRepo<C>
        + WorksetRepo<C>
        + ComicRepo<C>
        + ComicArchiveRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Send
        + Sync,
    O: ObjDept<TeamAvatar, C>
        + ObjDept<ComicCover, C>
        + ObjDept<PageImage, C>
        + Send
        + Sync,
{
    let member_info = FindMemberInfo::UserTeam {
        user_id: &token.user_id,
        team_id: &id,
    }
    .run_on(repo)
    .await?;

    let Some(member_info) = member_info else {
        //
        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: trl("error-team-admin-required"),
        });
    };

    TeamPermComplex::ensure_user_can_delete(&member_info)?;

    nucl.coord(async move |context| {
        delete_cascade(repo, obj_dept, context, &id).await
    })
    .await?;

    accept(())
}

// Delete a team subtree inside the caller-owned transaction.
async fn delete_cascade<C, R, O>(
    repo: &R,
    obj_dept: &O,
    context: &mut C,
    id: &str,
) -> BaseRest<()>
where
    C: Context,
    R: TeamRepo<C>
        + WorksetRepo<C>
        + ComicRepo<C>
        + ComicArchiveRepo<C>
        + ChapterRepo<C>
        + ChapterWorkflowRecordRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + MemberRepo<C>
        + TermbaseRepo<C>
        + TermRepo<C>
        + Sync,
    O: ObjDept<TeamAvatar, C>
        + ObjDept<ComicCover, C>
        + ObjDept<PageImage, C>
        + Sync,
{
    let team_info = GetTeamInfoExcluded::Id { id }
        .step_on(repo, context)
        .await?;

    delete_team_cascade(repo, context, &team_info.id).await?;

    let workset_infos = ListWorksetInfosExcluded {
        team_id: &team_info.id,
    }
    .step_on(repo, context)
    .await?;

    for workset_info in workset_infos {
        //
        delete_workset_cascade(repo, obj_dept, context, &workset_info.id)
            .await?;
    }

    DeleteObjs::<TeamAvatar>::new(std::slice::from_ref(&team_info.id))
        .step_on(obj_dept, context)
        .await
        .map_err(BaseError::from)?;

    let member_infos = ListMemberInfosExcluded::Team {
        team_id: &team_info.id,
    }
    .step_on(repo, context)
    .await?;

    for member_info in member_infos {
        //
        DeleteMember {
            id: &member_info.id,
        }
        .step_on(repo, context)
        .await?;
    }

    DeleteTeam { id: &team_info.id }
        .step_on(repo, context)
        .await?;

    accept(())
}
