//! In-memory terminology-base repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::termbase::{TermbaseInfo, TermbaseInfoListSpec};
use crate::part::repo::oper::termbase::{
    CreateTermbase, DeleteTermbase, GetTermbaseInfo, GetTermbaseInfoExcluded,
    ListTermbaseInfos, ListTermbaseInfosExcluded, TouchTermbase,
    UpdateTermbase, UpdateTermbaseTermCount,
};
use crate::part::repo::termbase::TermbaseRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseResult, accept};

impl TermbaseRepo<MockContext> for Mock {}

fn get_info(state: &MockState, id: &str) -> BaseResult<TermbaseInfo> {
    state
        .termbases
        .iter()
        .find(|termbase_info| termbase_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-termbase-not-found"))
}

fn same_owner(
    termbase_info: &TermbaseInfo,
    team_id: Option<&str>,
    comic_id: Option<&str>,
) -> bool {
    termbase_info.team_id.as_deref() == team_id
        && termbase_info.comic_id.as_deref() == comic_id
}

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

fn page_infos(
    mut termbase_infos: Vec<TermbaseInfo>,
    offset: u32,
    limit: u32,
) -> Vec<TermbaseInfo> {
    //
    termbase_infos
        .sort_by(|left, right| right.updated_at.cmp(&left.updated_at));

    termbase_infos
        .into_iter()
        .skip(offset as usize)
        .take(limit as usize)
        .collect()
}

fn list_infos(
    state: &MockState,
    spec: &TermbaseInfoListSpec,
) -> Vec<TermbaseInfo> {
    //
    let (mut termbase_infos, fuzzy_name, offset, limit) = match spec {
        //
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
        let fuzzy_name = fuzzy_name.to_lowercase();

        termbase_infos.retain(|termbase_info| {
            termbase_info.name.to_lowercase().contains(&fuzzy_name)
        });
    }

    page_infos(termbase_infos, offset, limit)
}

impl<'a> Run<GetTermbaseInfo<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseResult<TermbaseInfo> {
        //
        let state = self.state.lock().unwrap();

        get_info(&state, oper.id)
    }
}

impl<'a> Run<ListTermbaseInfos<'a>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListTermbaseInfos<'a>,
    ) -> BaseResult<Vec<TermbaseInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept(list_infos(&state, oper.spec))
    }
}

impl<'a> Step<CreateTermbase<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateTermbase<'a>,
    ) -> BaseResult<TermbaseInfo> {
        //
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTermbaseInfo<'a>,
    ) -> BaseResult<TermbaseInfo> {
        get_info(&context.state, oper.id)
    }
}

impl<'a> Step<GetTermbaseInfoExcluded<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetTermbaseInfoExcluded<'a>,
    ) -> BaseResult<TermbaseInfo> {
        get_info(&context.state, oper.id)
    }
}

impl<'a> Step<ListTermbaseInfosExcluded<'a>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListTermbaseInfosExcluded<'a>,
    ) -> BaseResult<Vec<TermbaseInfo>> {
        //
        let termbase_infos = context
            .state
            .termbases
            .iter()
            .filter(|termbase_info| match oper {
                //
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTermbase<'a>,
    ) -> BaseResult<()> {
        //
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateTermbaseTermCount<'a>,
    ) -> BaseResult<()> {
        //
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &TouchTermbase<'a>,
    ) -> BaseResult<()> {
        //
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
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteTermbase<'a>,
    ) -> BaseResult<()> {
        //
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
