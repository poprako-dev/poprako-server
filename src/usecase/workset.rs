//! Workset use cases — create, read, update, list, and deletion.

use poprako_orchestra::{Nucl, run_proxy};

use poprako_util::page::Page;

use crate::complex::workset::{WorksetComplex, WorksetPermComplex};
use crate::data::workset::{
    CreateWorksetParams, CreateWorksetPayload, ListWorksetInfosParams,
    UpdateWorksetInfoParams, WorksetInfoVal,
};
use crate::model::user::UserToken;
use crate::model::workset::{WorksetEntry, WorksetInfoUpdate};
use crate::part::prom::Prom;
use crate::part::repo::assignment::AssignmentRepo;
use crate::part::repo::assignment_invitation::AssignmentInvitationRepo;
use crate::part::repo::chapter::ChapterRepo;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::AllocateTeamWorksetIndex;
use crate::part::repo::oper::workset::{
    CreateWorkset, GetWorksetInfo, ListWorksetInfos, UpdateWorkset,
};
use crate::part::repo::page::PageRepo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::unit::UnitRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{RegularError, RegularResult};

#[cfg(test)]
pub mod tests;

/// Creates a new workset inside a team.
pub async fn create<N, C, R>(
    nucl: &N,
    repo: &R,
    token: UserToken,
    params: CreateWorksetParams,
) -> RegularResult<CreateWorksetPayload>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: TeamRepo<C> + WorksetRepo<C> + MemberRepo<C> + Send + Sync,
{
    WorksetPermComplex::ensure_user_can_create(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let workset_id = nucl
        .coord(async move |context| -> RegularResult<String> {
            //
            let index = repo
                .step(
                    context,
                    &AllocateTeamWorksetIndex {
                        id: &params.team_id,
                    },
                )
                .await?;

            let workset_entry = WorksetEntry {
                id: WorksetComplex::gen_id(),
                team_id: params.team_id,
                index,
                name: params.name,
                description: params.description,
            };

            let workset_info = repo
                .step(
                    context,
                    &CreateWorkset {
                        entry: &workset_entry,
                    },
                )
                .await?;

            Ok(workset_info.id)
        })
        .await?;

    Ok(CreateWorksetPayload { id: workset_id })
}

/// Fetches a workset by ID.
pub async fn get_info<C, R>(
    repo: &R,
    token: UserToken,
    id: String,
) -> RegularResult<WorksetInfoVal>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_get_info(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    let workset_info = repo.run(&GetWorksetInfo { id: &id }).await?;

    Ok(workset_info.into())
}

/// Lists worksets for a team.
pub async fn list_infos<C, R>(
    repo: &R,
    token: UserToken,
    params: ListWorksetInfosParams,
) -> RegularResult<Vec<WorksetInfoVal>>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_list_infos(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let workset_infos = repo
        .run(&ListWorksetInfos {
            team_id: &params.team_id,
            page: Some(Page {
                offset: params.offset,
                limit: params.limit,
            }),
        })
        .await?;

    Ok(workset_infos.into_iter().map(Into::into).collect())
}

/// Updates a workset's name and description.
pub async fn update_info<C, R>(
    repo: &R,
    token: UserToken,
    params: UpdateWorksetInfoParams,
) -> RegularResult<()>
where
    R: WorksetRepo<C> + MemberRepo<C> + Sync,
{
    WorksetPermComplex::ensure_user_can_update_info(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.id,
    )
    .await?;

    let workset_info_update = WorksetInfoUpdate {
        id: params.id,
        name: params.name,
        description: params.description,
    };

    repo.run(&UpdateWorkset {
        update: &workset_info_update,
    })
    .await?;

    Ok(())
}

/// Deletes a workset and its child data.
pub async fn delete<N, C, R, P>(
    nucl: &N,
    repo: &R,
    prom: &P,
    token: UserToken,
    id: String,
) -> RegularResult<()>
where
    N: Nucl<Context = C, Error = RegularError>,
    C: Send,
    R: WorksetRepo<C>
        + ComicRepo<C>
        + MemberRepo<C>
        + ChapterRepo<C>
        + PageRepo<C>
        + AssignmentInvitationRepo<C>
        + AssignmentRepo<C>
        + UnitRepo<C>
        + Send
        + Sync,
    P: Prom<C> + Send + Sync,
{
    WorksetPermComplex::ensure_user_can_delete(
        &mut run_proxy! {
            repo =>
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &id,
    )
    .await?;

    nucl.coord(async move |context| -> RegularResult<()> {
        //
        WorksetComplex::delete_cascade(repo, prom, context, &id).await?;

        Ok(())
    })
    .await?;

    Ok(())
}
