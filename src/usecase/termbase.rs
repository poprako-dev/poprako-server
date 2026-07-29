//! Terminology-base use cases.

use poprako_orchestra::{
    Nucl, OperRun as _, OperStep as _, run_proxy, step_proxy,
};
use tracing::instrument;

use crate::complex::termbase::{TermbaseComplex, TermbasePermComplex};
use crate::data::termbase::{
    CreateTermbaseParams, CreateTermbasePayload, ListComicTermbaseInfosParams,
    ListTeamTermbaseInfosParams, TermbaseInfoVal, UpdateTermbaseInfoParams,
};
use crate::model::termbase::TermbaseInfoListSpec;
use crate::model::user::UserToken;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::{GetComicInfo, GetComicInfoExcluded};
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::team::LockTeam;
use crate::part::repo::oper::term::DeleteTerms;
#[allow(unused_imports)]
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, UpdateTermbase,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, accept};

#[cfg(test)]
// Unit tests for terminology base definitions and search access.
mod tests;

/// Creates a terminology base scoped to a team or comic.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: CreateTermbaseParams,
) -> BaseRest<CreateTermbasePayload>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TeamRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + TermbaseRepo<C>
        + Send
        + Sync,
{
    let termbase_entry = TermbaseComplex::build_entry(
        params.team_id,
        params.comic_id,
        params.name,
        params.description,
        token.user_id.clone(),
    )?;

    let termbase_id = nucl
        .coord(async move |context| {
            //
            let team_id =
                match (&termbase_entry.team_id, &termbase_entry.comic_id) {
                    //
                    (Some(team_id), None) => {
                        //
                        LockTeam { id: team_id }.step_on(repo, context).await?;

                        team_id.clone()
                    }

                    (None, Some(comic_id)) => {
                        //
                        let comic_info = GetComicInfoExcluded {
                            id: comic_id,
                            incls: &[],
                        }
                        .step_on(repo, context)
                        .await?;

                        let workset_info = GetWorksetInfo {
                            id: &comic_info.workset_id,
                        }
                        .step_on(repo, context)
                        .await?;

                        workset_info.team_id
                    }

                    _ => unreachable!(),
                };

            TermbasePermComplex::ensure_user_can_write_team(
                &mut step_proxy! {
                    context;
                    repo => for<'a> FindMemberInfo<'a>;
                },
                &token.user_id,
                &team_id,
            )
            .await?;

            let termbase_info = CreateTermbase {
                entry: &termbase_entry,
            }
            .step_on(repo, context)
            .await?;

            accept(termbase_info.id)
        })
        .await?;

    accept(CreateTermbasePayload { id: termbase_id })
}

/// Fetches a terminology base by ID.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<TermbaseInfoVal>
where
    R: TermbaseRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    let termbase_info = GetTermbaseInfo { id: &id }.run_on(repo).await?;

    TermbasePermComplex::ensure_user_can_read(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &termbase_info,
    )
    .await?;

    accept(termbase_info.into())
}

/// Lists terminology bases directly owned by a team.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn list_team_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: ListTeamTermbaseInfosParams,
) -> BaseRest<Vec<TermbaseInfoVal>>
where
    R: TermbaseRepo<C> + MemberRepo<C> + Sync,
{
    TermbasePermComplex::ensure_user_can_read_team(
        &mut run_proxy! {
            repo => for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &params.team_id,
    )
    .await?;

    let termbase_info_list_spec = TermbaseInfoListSpec::Team {
        team_id: params.team_id,
        fuzzy_name: TermbaseComplex::normalize_fuzzy_name(params.fuzzy_name),
        offset: params.offset,
        limit: params.limit,
    };

    let termbase_infos = ListTermbaseInfos {
        spec: &termbase_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(termbase_infos.into_iter().map(Into::into).collect())
}

/// Lists team and comic terminology bases visible from a comic.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn list_comic_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: ListComicTermbaseInfosParams,
) -> BaseRest<Vec<TermbaseInfoVal>>
where
    R: TermbaseRepo<C> + ComicRepo<C> + WorksetRepo<C> + MemberRepo<C> + Sync,
{
    let team_id = TermbasePermComplex::resolve_team_id_from_comic(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &params.comic_id,
    )
    .await?;

    TermbasePermComplex::ensure_user_can_read_team(
        &mut run_proxy! {
            repo =>
                for<'a, 'b> GetComicInfo<'a, 'b>,
                for<'a> GetWorksetInfo<'a>,
                for<'a> FindMemberInfo<'a>;
        },
        &token.user_id,
        &team_id,
    )
    .await?;

    let termbase_info_list_spec = TermbaseInfoListSpec::Comic {
        team_id,
        comic_id: params.comic_id,
        fuzzy_name: TermbaseComplex::normalize_fuzzy_name(params.fuzzy_name),
        offset: params.offset,
        limit: params.limit,
    };

    let termbase_infos = ListTermbaseInfos {
        spec: &termbase_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(termbase_infos.into_iter().map(Into::into).collect())
}

/// Replaces a terminology base's name and description.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: UpdateTermbaseInfoParams,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TermbaseRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    let termbase_info_update = TermbaseComplex::build_update(
        params.id,
        params.name,
        params.description,
    )?;

    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded {
            id: &termbase_info_update.id,
        }
        .step_on(repo, context)
        .await?;

        TermbasePermComplex::ensure_user_can_write(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a, 'b> GetComicInfo<'a, 'b>,
                    for<'a> GetWorksetInfo<'a>,
                    for<'a> FindMemberInfo<'a>;
            },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        UpdateTermbase {
            update: &termbase_info_update,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a terminology base and all child terms.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    N: Nucl<Context = C, Error = BaseError>,
    C: Send,
    R: TermbaseRepo<C>
        + TermRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded { id: &id }
            .step_on(repo, context)
            .await?;

        TermbasePermComplex::ensure_user_can_write(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a, 'b> GetComicInfo<'a, 'b>,
                    for<'a> GetWorksetInfo<'a>,
                    for<'a> FindMemberInfo<'a>;
            },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        TermbaseComplex::delete_cascade(
            &mut step_proxy! {
                context;
                repo =>
                    for<'a> GetTermbaseInfoExcluded<'a>,
                    for<'a> DeleteTerms<'a>,
                    for<'a> DeleteTermbase<'a>;
            },
            &termbase_info.id,
        )
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
