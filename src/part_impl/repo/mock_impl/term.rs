//! In-memory terminology-entry repository operations.

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::term::TermInfo;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::term::{
    CreateTerm, DeleteTerm, DeleteTerms, GetTermInfo, GetTermInfoExcluded,
    ListTermInfos, LockTerm, UpdateTerm, UpsertTerms,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `get_info`.
fn get_info(state: &MockState, id: &str) -> BaseRest<TermInfo> {
    //
    state
        .terms
        .iter()
        .find(|term_info| term_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-term-not-found"))
}

// Internal implementation of `source_conflicts`.
fn source_conflicts(
    state: &MockState,
    id: Option<&str>,
    termbase_id: &str,
    source: &str,
) -> bool {
    //
    state.terms.iter().any(|term_info| {
        //
        term_info.id != id.unwrap_or_default()
            && term_info.termbase_id == termbase_id
            && term_info.source.to_lowercase() == source.to_lowercase()
    })
}

// Internal implementation of `list_infos`.
fn list_infos(
    state: &MockState,
    termbase_id: &str,
    fuzzy_source: Option<&str>,
    offset: u32,
    limit: u32,
) -> Vec<TermInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut term_infos = state
        .terms
        .iter()
        .filter(|term_info| term_info.termbase_id == termbase_id)
        .cloned()
        .collect::<Vec<_>>();

    if let Some(fuzzy_source) = fuzzy_source {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let fuzzy_source = fuzzy_source.to_lowercase();

        term_infos.retain(|term_info| {
            term_info.source.to_lowercase().contains(&fuzzy_source)
        });
    }

    term_infos.sort_by_key(|right| Reverse(right.updated_at));

    term_infos
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect()
}

// List every term belonging to one terminology base in stable source order.
fn list_all_infos(state: &MockState, termbase_id: &str) -> Vec<TermInfo> {
    //
    let mut term_infos = state
        .terms
        .iter()
        .filter(|term_info| term_info.termbase_id == termbase_id)
        .cloned()
        .collect::<Vec<_>>();

    term_infos.sort_by(|left, right| {
        //
        left.source
            .cmp(&right.source)
            .then_with(|| left.id.cmp(&right.id))
    });

    term_infos
}

impl<'a> Run<GetTermInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Resolve one term by identifier.
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetTermInfo<'a>) -> BaseRest<TermInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_info(&state, oper.id)
    }
}

impl<'a> Run<ListTermInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route list variants against the committed mock state.
    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &ListTermInfos<'a>) -> BaseRest<Vec<TermInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        let term_infos = match oper {
            //
            ListTermInfos::Query {
                termbase_id,
                fuzzy_source,
                offset,
                limit,
            } => {
                list_infos(&state, termbase_id, *fuzzy_source, *offset, *limit)
            }

            ListTermInfos::Termbase { termbase_id } => {
                list_all_infos(&state, termbase_id)
            }
        };

        accept(term_infos)
    }
}

impl<'a> Step<CreateTerm<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTerm<'a>,
    ) -> BaseRest<TermInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if source_conflicts(
            &context.state,
            None,
            &oper.entry.termbase_id,
            &oper.entry.source,
        ) {
            return Err(expected("error-already-exists"));
        }

        if !context
            .state
            .termbases
            .iter()
            .any(|termbase_info| termbase_info.id == oper.entry.termbase_id)
        {
            return Err(expected("error-termbase-not-found"));
        }

        let time = now();

        let term_info = TermInfo {
            id: oper.entry.id.clone(),
            termbase_id: oper.entry.termbase_id.clone(),
            source: oper.entry.source.clone(),
            targets: oper.entry.targets.clone(),
            comment: oper.entry.comment.clone(),
            creator_id: oper.entry.creator_id.clone(),
            created_at: time,
            updated_at: time,
        };

        context.state.terms.push(term_info.clone());

        accept(term_info)
    }
}

impl<'a> Step<ListTermInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Route list variants inside the mock transaction snapshot.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListTermInfos<'a>,
    ) -> BaseRest<Vec<TermInfo>> {
        //
        let term_infos = match oper {
            //
            ListTermInfos::Query {
                termbase_id,
                fuzzy_source,
                offset,
                limit,
            } => list_infos(
                &context.state,
                termbase_id,
                *fuzzy_source,
                *offset,
                *limit,
            ),

            ListTermInfos::Termbase { termbase_id } => {
                list_all_infos(&context.state, termbase_id)
            }
        };

        accept(term_infos)
    }
}

impl<'a> Step<GetTermInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTermInfoExcluded<'a>,
    ) -> BaseRest<TermInfo> {
        get_info(&context.state, oper.id)
    }
}

impl<'a> Step<LockTerm<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &LockTerm<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        get_info(&context.state, oper.id)?;

        accept(())
    }
}

impl<'a> Step<UpdateTerm<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTerm<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let current = get_info(&context.state, &oper.update.id)?;

        if source_conflicts(
            &context.state,
            Some(&oper.update.id),
            &current.termbase_id,
            &oper.update.source,
        ) {
            return Err(expected("error-already-exists"));
        }

        let term_info = context
            .state
            .terms
            .iter_mut()
            .find(|term_info| term_info.id == oper.update.id)
            .ok_or_else(|| expected("error-term-not-found"))?;

        term_info.source = oper.update.source.clone();

        term_info.targets = oper.update.targets.clone();

        term_info.comment = oper.update.comment.clone();

        term_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<UpsertTerms<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Apply imported entries and updates inside the mock transaction.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpsertTerms<'a>,
    ) -> BaseRest<()> {
        //
        for entry in oper.entries {
            //
            if source_conflicts(
                &context.state,
                None,
                &entry.termbase_id,
                &entry.source,
            ) {
                return Err(expected("error-already-exists"));
            }

            let time = now();

            context.state.terms.push(TermInfo {
                id: entry.id.clone(),
                termbase_id: entry.termbase_id.clone(),
                source: entry.source.clone(),
                targets: entry.targets.clone(),
                comment: entry.comment.clone(),
                creator_id: entry.creator_id.clone(),
                created_at: time,
                updated_at: time,
            });
        }

        for update in oper.updates {
            //
            let current = get_info(&context.state, &update.id)?;

            if source_conflicts(
                &context.state,
                Some(&update.id),
                &current.termbase_id,
                &update.source,
            ) {
                return Err(expected("error-already-exists"));
            }

            let term_info = context
                .state
                .terms
                .iter_mut()
                .find(|term_info| term_info.id == update.id)
                .ok_or_else(|| expected("error-term-not-found"))?;

            term_info.source = update.source.clone();

            term_info.targets = update.targets.clone();

            term_info.comment = update.comment.clone();

            term_info.updated_at = now();
        }

        accept(())
    }
}

impl<'a> Step<DeleteTerm<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTerm<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let position = context
            .state
            .terms
            .iter()
            .position(|term_info| term_info.id == oper.id)
            .ok_or_else(|| expected("error-term-not-found"))?;

        context.state.terms.remove(position);

        accept(())
    }
}

impl<'a> Step<DeleteTerms<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = RepeatableRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTerms<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        context
            .state
            .terms
            .retain(|term_info| term_info.termbase_id != oper.termbase_id);

        accept(())
    }
}
