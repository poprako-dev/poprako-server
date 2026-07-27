//! In-memory terminology-base repository operations.

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::termbase::{TermbaseInfo, TermbaseInfoListSpec};
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `same_owner`.
fn same_owner(
    termbase_info: &TermbaseInfo,
    team_id: Option<&str>,
    comic_id: Option<&str>,
) -> bool {
    termbase_info.team_id.as_deref() == team_id
        && termbase_info.comic_id.as_deref() == comic_id
}

// Internal implementation of `page_infos`.
fn page_infos(
    mut termbase_infos: Vec<TermbaseInfo>,
    offset: u32,
    limit: u32,
) -> Vec<TermbaseInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    termbase_infos.sort_by_key(|right| Reverse(right.updated_at));

    termbase_infos
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect()
}

// Internal implementation of `name_conflicts`.
fn name_conflicts(
    state: &MockState,
    id: Option<&str>,
    team_id: Option<&str>,
    comic_id: Option<&str>,
    name: &str,
) -> bool {
    state.termbases.iter().any(|termbase_info| {
        termbase_info.id != id.unwrap_or_default()
            && same_owner(termbase_info, team_id, comic_id)
            && termbase_info.name.to_lowercase() == name.to_lowercase()
    })
}

// Internal implementation of `list_infos`.
fn list_infos(
    state: &MockState,
    spec: &TermbaseInfoListSpec,
) -> Vec<TermbaseInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let (mut termbase_infos, fuzzy_name, offset, limit) = match spec {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        TermbaseInfoListSpec::Team {
            team_id,
            fuzzy_name,
            offset,
            limit,
        } => (
            state
                .termbases
                .iter()
                .filter(|termbase_info| {
                    termbase_info.team_id.as_deref() == Some(team_id)
                })
                .cloned()
                .collect::<Vec<_>>(),
            fuzzy_name,
            *offset,
            *limit,
        ),

        TermbaseInfoListSpec::Comic {
            team_id,
            comic_id,
            fuzzy_name,
            offset,
            limit,
        } => (
            state
                .termbases
                .iter()
                .filter(|termbase_info| {
                    termbase_info.team_id.as_deref() == Some(team_id)
                        || termbase_info.comic_id.as_deref() == Some(comic_id)
                })
                .cloned()
                .collect::<Vec<_>>(),
            fuzzy_name,
            *offset,
            *limit,
        ),
    };

    if let Some(fuzzy_name) = fuzzy_name {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let fuzzy_name = fuzzy_name.to_lowercase();

        termbase_infos.retain(|termbase_info| {
            termbase_info.name.to_lowercase().contains(&fuzzy_name)
        });
    }

    page_infos(termbase_infos, offset, limit)
}

// Resolve one termbase by id and return it with expected-args missing error.
fn get_info(state: &MockState, id: &str) -> BaseRest<TermbaseInfo> {
    state
        .termbases
        .iter()
        .find(|termbase_info| termbase_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-termbase-not-found"))
}

impl<'a> Run<GetTermbaseInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseRest<TermbaseInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_info(&state, oper.id)
    }
}

impl<'a> Run<ListTermbaseInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListTermbaseInfos<'a>,
    ) -> BaseRest<Vec<TermbaseInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_infos(&state, oper.spec))
    }
}

impl<'a> Step<CreateTermbase<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTermbase<'a>,
    ) -> BaseRest<TermbaseInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if name_conflicts(
            &context.state,
            None,
            oper.entry.team_id.as_deref(),
            oper.entry.comic_id.as_deref(),
            &oper.entry.name,
        ) {
            return Err(expected("error-already-exists"));
        }

        let time = now();

        let termbase_info = TermbaseInfo {
            id: oper.entry.id.clone(),
            team_id: oper.entry.team_id.clone(),
            comic_id: oper.entry.comic_id.clone(),
            name: oper.entry.name.clone(),
            description: oper.entry.description.clone(),
            term_count: 0,
            creator_id: oper.entry.creator_id.clone(),
            created_at: time,
            updated_at: time,
        };

        context.state.termbases.push(termbase_info.clone());

        accept(termbase_info)
    }
}

impl<'a> Step<GetTermbaseInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseRest<TermbaseInfo> {
        get_info(&context.state, oper.id)
    }
}

impl<'a> Step<GetTermbaseInfoExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTermbaseInfoExcluded<'a>,
    ) -> BaseRest<TermbaseInfo> {
        get_info(&context.state, oper.id)
    }
}

impl<'a> Step<ListTermbaseInfosExcluded<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListTermbaseInfosExcluded<'a>,
    ) -> BaseRest<Vec<TermbaseInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let termbase_infos = context
            .state
            .termbases
            .iter()
            .filter(|termbase_info| match oper {
                //
                // Internal implementation detail.
                // Internal implementation detail.
                ListTermbaseInfosExcluded::Team { team_id } => {
                    termbase_info.team_id.as_deref() == Some(*team_id)
                }

                ListTermbaseInfosExcluded::Comic { comic_id } => {
                    termbase_info.comic_id.as_deref() == Some(*comic_id)
                }
            })
            .cloned()
            .collect();

        accept(termbase_infos)
    }
}

impl<'a> Step<UpdateTermbase<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTermbase<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let current = get_info(&context.state, &oper.update.id)?;

        if name_conflicts(
            &context.state,
            Some(&oper.update.id),
            current.team_id.as_deref(),
            current.comic_id.as_deref(),
            &oper.update.name,
        ) {
            return Err(expected("error-already-exists"));
        }

        let termbase_info = context
            .state
            .termbases
            .iter_mut()
            .find(|termbase_info| termbase_info.id == oper.update.id)
            .ok_or_else(|| expected("error-termbase-not-found"))?;

        termbase_info.name = oper.update.name.clone();

        termbase_info.description = oper.update.description.clone();

        termbase_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<UpdateTermbaseTermCount<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTermbaseTermCount<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let termbase_info = context
            .state
            .termbases
            .iter_mut()
            .find(|termbase_info| termbase_info.id == oper.id)
            .ok_or_else(|| expected("error-termbase-not-found"))?;

        let term_count = termbase_info.term_count + oper.delta;

        if term_count < 0 {
            return Err(expected("error-invalid-term-count"));
        }

        termbase_info.term_count = term_count;

        termbase_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<TouchTermbase<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &TouchTermbase<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let termbase_info = context
            .state
            .termbases
            .iter_mut()
            .find(|termbase_info| termbase_info.id == oper.id)
            .ok_or_else(|| expected("error-termbase-not-found"))?;

        termbase_info.updated_at = now();

        accept(())
    }
}

impl<'a> Step<DeleteTermbase<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTermbase<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let position = context
            .state
            .termbases
            .iter()
            .position(|termbase_info| termbase_info.id == oper.id)
            .ok_or_else(|| expected("error-termbase-not-found"))?;

        context.state.termbases.remove(position);

        accept(())
    }
}
