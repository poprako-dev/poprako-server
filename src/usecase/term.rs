//! Terminology-entry use cases.

#[cfg(test)]
// Unit tests for term lifecycle, ownership, and conflict guards.
mod tests;

use poprako_orchestra::{AtLeast, Context, Nucl, OperRun as _, OperStep as _};
use tracing::instrument;

use crate::complex::term::TermComplex;
use crate::complex::termbase::{TermbaseComplex, TermbasePermComplex};
use crate::data::instr::term::{
    CreateTermInstr, ListTermInfosInstr, UpdateTermInfoInstr,
};
use crate::data::val::term::CreateTermVal;
use crate::data::view::term::TermInfoView;
use crate::model::read::spec::term::TermListSpec;
use crate::model::shared::user::UserToken;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::member::MemberRepo;
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, GetTermInfo, ListTermInfos, LockTerm, UpdateTerm,
};
use crate::part::repo::oper::termbase::{
    GetTermbaseInfo, GetTermbaseInfoExcluded, TouchTermbase,
    UpdateTermbaseTermCount,
};
use crate::part::repo::team::TeamRepo;
use crate::part::repo::term::TermRepo;
use crate::part::repo::termbase::TermbaseRepo;
use crate::result::{BaseError, BaseRest, accept};
use crate::usecase::internal::member::MemberLoader;
use crate::usecase::internal::util::LoadMode;

/// Creates a terminology entry inside a terminology base.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn create<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: CreateTermInstr,
) -> BaseRest<CreateTermVal>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: TermbaseRepo<C>
        + TermRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    let term_entry = TermComplex::build_entry(
        instr.termbase_id,
        instr.source,
        instr.targets,
        instr.comment,
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

            let member_info = MemberLoader::load_info_from_termbase(
                repo,
                LoadMode::Step { context },
                &token.user_id,
                &termbase_info,
            )
            .await?;

            TermbasePermComplex::ensure_user_can_write(
                &member_info,
                &termbase_info,
            )?;

            TermbaseComplex::ensure_term_capacity(termbase_info.term_count, 1)?;

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

    accept(CreateTermVal { id: term_id })
}

/// Fetches a terminology entry by ID.
#[instrument(level = "info", skip(repo))]
pub async fn get_info<C, R>(
    (repo,): (&R,),
    token: UserToken,
    id: String,
) -> BaseRest<TermInfoView>
where
    C: Context,
    R: TermbaseRepo<C> + TermRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let term_info = GetTermInfo { id: &id }.run_on(repo).await?;

    let termbase_info = GetTermbaseInfo {
        id: &term_info.termbase_id,
    }
    .run_on(repo)
    .await?;

    let member_info = MemberLoader::load_info_from_termbase(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &termbase_info,
    )
    .await?;

    TermbasePermComplex::ensure_user_can_read(&member_info, &termbase_info)?;

    accept(term_info.into())
}

/// Lists terminology entries inside one terminology base.
#[instrument(level = "info", skip(repo))]
pub async fn list_infos<C, R>(
    (repo,): (&R,),
    token: UserToken,
    instr: ListTermInfosInstr,
) -> BaseRest<Vec<TermInfoView>>
where
    C: Context,
    R: TermbaseRepo<C> + TermRepo<C> + TeamRepo<C> + MemberRepo<C> + Sync,
{
    let termbase_info = GetTermbaseInfo {
        id: &instr.termbase_id,
    }
    .run_on(repo)
    .await?;

    let member_info = MemberLoader::load_info_from_termbase(
        repo,
        LoadMode::<C>::Run,
        &token.user_id,
        &termbase_info,
    )
    .await?;

    TermbasePermComplex::ensure_user_can_read(&member_info, &termbase_info)?;

    let term_info_list_spec = TermListSpec {
        termbase_id: instr.termbase_id,
        fuzzy_source: TermComplex::normalize_fuzzy_source(instr.fuzzy_source),
        offset: instr.offset,
        limit: instr.limit,
    };

    let term_infos = ListTermInfos::Spec {
        spec: &term_info_list_spec,
    }
    .run_on(repo)
    .await?;

    accept(term_infos.into_iter().map(Into::into).collect())
}

/// Replaces a terminology entry's source, targets, and comment.
#[instrument(level = "info", skip(nucl, repo))]
pub async fn update_info<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    instr: UpdateTermInfoInstr,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: TermbaseRepo<C>
        + TermRepo<C>
        + TeamRepo<C>
        + MemberRepo<C>
        + Send
        + Sync,
{
    let term_info_update = TermComplex::build_update(
        instr.id,
        instr.source,
        instr.targets,
        instr.comment,
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

        let member_info = MemberLoader::load_info_from_termbase(
            repo,
            LoadMode::Step { context },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        TermbasePermComplex::ensure_user_can_write(
            &member_info,
            &termbase_info,
        )?;

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
#[instrument(level = "info", skip(nucl, repo))]
pub async fn delete<N, C, R>(
    (nucl, repo): (&N, &R),
    token: UserToken,
    id: String,
) -> BaseRest<()>
where
    C: Context + Send,
    N: Nucl<Context = C, Error = BaseError>,
    C::Level: AtLeast<RepeatableRead>,
    R: TermbaseRepo<C>
        + TermRepo<C>
        + TeamRepo<C>
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

        let member_info = MemberLoader::load_info_from_termbase(
            repo,
            LoadMode::Step { context },
            &token.user_id,
            &termbase_info,
        )
        .await?;

        TermbasePermComplex::ensure_user_can_write(
            &member_info,
            &termbase_info,
        )?;

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
