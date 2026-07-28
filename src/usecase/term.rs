//! Terminology-entry use cases.

use poprako_orchestra::{
    Nucl, OperRun as _, OperStep as _, run_proxy, step_proxy,
};
use tracing::instrument;

use crate::complex::term::TermComplex;
use crate::complex::termbase::TermbasePermComplex;
use crate::data::term::{
    CreateTermParams, CreateTermPayload, ListTermInfosParams, TermInfoVal,
    UpdateTermInfoParams,
};
use crate::model::read::spec::term::TermListSpec;
use crate::model::shared::user::UserToken;
use crate::part::repo::comic::ComicRepo;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::comic::GetComicInfo;
use crate::part::repo::oper::member::FindMemberInfo;
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, GetTermInfo, ListTermInfos, LockTerm, UpdateTerm,
};
use crate::part::repo::oper::termbase::{
    GetTermbaseInfo, GetTermbaseInfoExcluded, TouchTermbase,
    UpdateTermbaseTermCount,
};
use crate::part::repo::oper::workset::GetWorksetInfo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::part::repo::workset::WorksetRepo;
use crate::result::{BaseError, BaseRest, accept};

#[cfg(test)]
// Unit tests for term lifecycle, ownership, and conflict guards.
mod tests;

/// Creates a terminology entry inside a terminology base.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: CreateTermParams,
) -> BaseRest<CreateTermPayload>
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
    let term_entry = TermComplex::build_entry(
        params.termbase_id,
        params.source,
        params.targets,
        params.comment,
        token.user_id.clone(),
    )?;

    let term_id = nucl
        .coord(async move |context| {
            //
            let termbase_info = GetTermbaseInfoExcluded {
                id: &term_entry.termbase_id,
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

            let term_info = CreateTerm { entry: &term_entry }
                .step_on(repo, context)
                .await?;

            UpdateTermbaseTermCount {
                id: &termbase_info.id,
                delta: 1,
            }
            .step_on(repo, context)
            .await?;

            accept(term_info.id)
        })
        .await?;

    accept(CreateTermPayload { id: term_id })
}

/// Fetches a terminology entry by ID.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<TermInfoVal>
where
    R: TermbaseRepo<C>
        + TermRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Sync,
{
    let term_info = GetTermInfo { id: &id }.run_on(repo).await?;

    let termbase_info = GetTermbaseInfo {
        id: &term_info.termbase_id,
    }
    .run_on(repo)
    .await?;

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

    accept(term_info.into())
}

/// Lists terminology entries inside one terminology base.
#[instrument(level = "info", err(Debug), skip(repo))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    params: ListTermInfosParams,
) -> BaseRest<Vec<TermInfoVal>>
where
    R: TermbaseRepo<C>
        + TermRepo<C>
        + ComicRepo<C>
        + WorksetRepo<C>
        + MemberRepo<C>
        + Sync,
{
    let termbase_info = GetTermbaseInfo {
        id: &params.termbase_id,
    }
    .run_on(repo)
    .await?;

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

    let term_info_list_spec = TermListSpec {
        termbase_id: params.termbase_id,
        fuzzy_source: TermComplex::normalize_fuzzy_source(params.fuzzy_source),
        offset: params.offset,
        limit: params.limit,
    };

    let term_infos = ListTermInfos {
        spec: &term_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(term_infos.into_iter().map(Into::into).collect())
}

/// Replaces a terminology entry's source, targets, and comment.
#[instrument(level = "info", err(Debug), skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    params: UpdateTermInfoParams,
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
    let term_info_update = TermComplex::build_update(
        params.id,
        params.source,
        params.targets,
        params.comment,
    )?;

    let term_info = GetTermInfo {
        id: &term_info_update.id,
    }
    .run_on(repo)
    .await?;

    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded {
            id: &term_info.termbase_id,
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

        LockTerm {
            id: &term_info_update.id,
        }
        .step_on(repo, context)
        .await?;

        UpdateTerm {
            update: &term_info_update,
        }
        .step_on(repo, context)
        .await?;

        TouchTermbase {
            id: &termbase_info.id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}

/// Deletes a terminology entry.
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
    let term_info = GetTermInfo { id: &id }.run_on(repo).await?;

    nucl.coord(async move |context| {
        //
        let termbase_info = GetTermbaseInfoExcluded {
            id: &term_info.termbase_id,
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

        LockTerm { id: &term_info.id }
            .step_on(repo, context)
            .await?;

        DeleteTerm { id: &term_info.id }
            .step_on(repo, context)
            .await?;

        UpdateTermbaseTermCount {
            id: &termbase_info.id,
            delta: -1,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await?;

    accept(())
}
