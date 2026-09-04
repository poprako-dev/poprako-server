//! In-memory workset repository operations.

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::read::proj::workset::WorksetInfo;
use crate::model::write::workset::WorksetRepl;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, GetWorksetInfo, ListWorksetInfos,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part_impl::repo::mock_impl::nucl::apply_signed_delta;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{BaseError, BaseRest, accept};

// Internal implementation of `get_workset_info`.
fn get_workset_info(state: &MockState, id: &str) -> BaseRest<WorksetInfo> {
    //
    state
        .worksets
        .iter()
        .find(|workset_info| {
            workset_info.id == id && !state.deleted_workset_ids.contains(id)
        })
        .cloned()
        .ok_or_else(|| expected("error-workset-not-found"))
}

// Internal implementation of `list_workset_infos`.
fn list_workset_infos(
    state: &MockState,
    oper: &ListWorksetInfos<'_>,
) -> Vec<WorksetInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let mut workset_infos = state
        .worksets
        .iter()
        .filter(|workset_info| workset_info.team_id == oper.team_id)
        .filter(|workset_info| {
            !state.deleted_workset_ids.contains(&workset_info.id)
        })
        .cloned()
        .collect::<Vec<_>>();

    workset_infos.sort_by_key(|workset_info| workset_info.index);

    let offset = oper.offset as usize;

    let limit = oper.limit.get() as usize;

    if offset >= workset_infos.len() {
        Vec::new()
    } else {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let end = std::cmp::min(offset + limit, workset_infos.len());

        workset_infos[offset..end].to_vec()
    }
}

// Internal implementation of `update_workset`.
fn update_workset(state: &mut MockState, update: &WorksetRepl) -> BaseRest<()> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state.deleted_workset_ids.contains(&update.id) {
        return Err(expected("error-workset-not-found"));
    }

    let workset_info = state
        .worksets
        .iter_mut()
        .find(|workset_info| workset_info.id == update.id)
        .ok_or_else(|| expected("error-workset-not-found"))?;

    workset_info.name = update.name.clone();

    workset_info.description = update.description.clone();

    workset_info.updated_at = now();

    accept(())
}

impl<'a> Run<GetWorksetInfo<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &GetWorksetInfo<'a>) -> BaseRest<WorksetInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        get_workset_info(&state, oper.id)
    }
}

impl<'a> Run<ListWorksetInfos<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListWorksetInfos<'a>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_workset_infos(&state, oper))
    }
}

impl<'a> Run<UpdateWorkset<'a>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(&self, oper: &UpdateWorkset<'a>) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let mut state = self.state.lock().unwrap();

        update_workset(&mut state, oper.update)
    }
}

impl<'a> Step<GetWorksetInfo<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetWorksetInfo<'a>,
    ) -> BaseRest<WorksetInfo> {
        get_workset_info(&context.state, oper.id)
    }
}

impl<'a> Step<ListWorksetInfos<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListWorksetInfos<'a>,
    ) -> BaseRest<Vec<WorksetInfo>> {
        accept(list_workset_infos(&context.state, oper))
    }
}

impl<'a> Step<CreateWorkset<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateWorkset<'a>,
    ) -> BaseRest<WorksetInfo> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if context
            .state
            .worksets
            .iter()
            .any(|workset_info| workset_info.id == oper.entry.id)
        {
            return Err(expected("error-already-exists"));
        }

        let time = now();

        let workset_info = WorksetInfo {
            id: oper.entry.id.clone(),
            team_id: oper.entry.team_id.clone(),
            index: oper.entry.index,
            name: oper.entry.name.clone(),
            description: oper.entry.description.clone(),
            comic_count: 0,
            created_at: time,
            updated_at: time,
        };

        context.state.worksets.push(workset_info.clone());

        accept(workset_info)
    }
}

impl<'a> Step<AllocWorksetComicIndex<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocWorksetComicIndex<'a>,
    ) -> BaseRest<usize> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        // verify the workset exists
        if context.state.deleted_workset_ids.contains(oper.id) {
            return Err(expected("error-workset-not-found"));
        }

        context
            .state
            .worksets
            .iter()
            .find(|ws| ws.id == oper.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;

        let index = context
            .state
            .comics
            .iter()
            .filter(|comic_info| comic_info.workset_id == oper.id)
            .count();

        accept(index)
    }
}

impl<'a> Step<UpdateWorksetComicCount<'a>, MockContext> for Mock {
    // Internal type alias for `Error`.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `step`.
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateWorksetComicCount<'a>,
    ) -> BaseRest<()> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        if context.state.deleted_workset_ids.contains(oper.id) {
            return Err(expected("error-workset-not-found"));
        }

        let workset_info = context
            .state
            .worksets
            .iter_mut()
            .find(|workset_info| workset_info.id == oper.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;

        apply_signed_delta(&mut workset_info.comic_count, oper.delta)?;

        workset_info.updated_at = now();

        accept(())
    }
}
