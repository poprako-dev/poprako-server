//! Non-transactional team read use cases.

use poprako_orchestra::{Context, OperRun as _, Run};
use tracing::instrument;

use poprako_obj_dept::oper::GenObjUrl;
use poprako_obj_dept::rest::ObjDeptError;
use poprako_util::i18n::trl;

use crate::complex::team::TeamPermComplex;
use crate::data::instr::team::ListTeamInfosInstr;
use crate::data::view::team::TeamInfoView;
use crate::model::read::spec::team::TeamListSpec;
use crate::model::shared::user::UserToken;
use crate::part::obj_dept::TeamAvatar;
use crate::part::repo::oper::team::{GetTeamInfo, ListTeamInfos};
use crate::part::repo::oper::user::GetUserInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::user::UserRepo;
use crate::result::{BaseError, BaseRest, ExpectedVariant, accept};
use crate::usecase::internal::util::collect_bounded;
use crate::usecase::view::team_info_view;

/// Fetches a team by ID with avatar URL resolution.
///
/// Non-transactional read.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `O` — Resolves the avatar signed URL through `ObjDept`.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn get_info<C, R, O>(
    (repo, obj_dept): (&R, &O),
    id: String,
) -> BaseRest<TeamInfoView>
where
    C: Context,
    R: TeamRepo<C>,
    O: for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError> + Sync,
{
    let team_info = GetTeamInfo::Id { id: &id }.run_on(repo).await?;

    team_info_view(obj_dept, team_info).await
}

/// Lists teams with pagination.
///
/// Non-transactional read. Each team's avatar URL is resolved individually.
///
/// # Type Parameters
///
/// * `C` — Context anchor.
/// * `R: TeamRepo<C>` — Team storage.
/// * `O` — Resolves avatar signed URLs through `ObjDept`.
#[instrument(level = "info", skip(repo, obj_dept))]
pub async fn list_infos<C, R, O>(
    (repo, obj_dept): (&R, &O),
    token: UserToken,
    // FIXME: use try_into()?
    instr: ListTeamInfosInstr,
) -> BaseRest<Vec<TeamInfoView>>
where
    C: Context,
    R: TeamRepo<C> + UserRepo<C> + Sync,
    O: for<'a> Run<GenObjUrl<'a, TeamAvatar>, Error = ObjDeptError> + Sync,
{
    if let Some(affected_user_id) = instr.user_id.as_deref()
        && affected_user_id != token.user_id
    {
        //
        let err_message = trl("error-forbidden");

        tracing::warn!(
            err_variant = ?ExpectedVariant::Perm,
            err_message = %err_message,
            user_id = %token.user_id,
            affected_user_id = %affected_user_id,
            "expected error: team listing ownership required",
        );

        return Err(BaseError::Expected {
            variant: ExpectedVariant::Perm,
            message: err_message,
        });
    }

    if instr.user_id.is_none() {
        //
        let user_info =
            GetUserInfo::Id { id: &token.user_id }.run_on(repo).await?;

        TeamPermComplex::ensure_user_can_list_infos(&user_info)?;
    }

    let team_info_list_spec = TeamListSpec {
        user_id: instr.user_id,
        offset: instr.offset,
        limit: instr.limit,
    };

    let team_infos = ListTeamInfos {
        spec: &team_info_list_spec,
    }
    .run_on(repo)
    .await?;

    let team_info_vals = collect_bounded(
        team_infos
            .into_iter()
            .map(|team_info| team_info_view(obj_dept, team_info)),
    )
    .await?;

    accept(team_info_vals)
}
