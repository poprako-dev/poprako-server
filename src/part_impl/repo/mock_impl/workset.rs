//! In-memory workset repository operations.

use poprako_orchestra::{Run, Step};

use tracing::instrument;

use crate::model::workset::{WorksetInfo, WorksetInfoUpdate};
use crate::part::repo::oper::workset::{
    AllocWorksetComicIndex, CreateWorkset, DeleteWorkset, GetWorksetInfo,
    GetWorksetInfoExcluded, ListWorksetInfos, ListWorksetInfosExcluded,
    UpdateWorkset, UpdateWorksetComicCount,
};
use crate::part::repo::workset::WorksetRepo;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, now,
};
use crate::result::{RegularError, RegularResult};

impl WorksetRepo<MockContext> for Mock {}

fn get_workset_info(state: &MockState, id: &str) -> RegularResult<WorksetInfo> {
    state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-workset-not-found"))
}

fn list_workset_infos(
    state: &MockState,
    oper: &ListWorksetInfos<'_>,
) -> Vec<WorksetInfo> {
    //
    let mut workset_infos = state
        .worksets
        .iter()
        .filter(|workset_info| workset_info.team_id == oper.team_id)
        .cloned()
        .collect::<Vec<_>>();

    workset_infos.sort_by_key(|workset_info| workset_info.index);

    let Some(page) = oper.page else {
        return workset_infos;
    };

    let offset = page.offset as usize;

    let limit = page.limit as usize;

    match offset >= workset_infos.len() {
        //
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, workset_infos.len());

            workset_infos[offset..end].to_vec()
        }
    }
}

fn update_workset(
    state: &mut MockState,
    update: &WorksetInfoUpdate,
) -> RegularResult<()> {
    //
    let workset_info = state
        .worksets
        .iter_mut()
        .find(|workset_info| workset_info.id == update.id)
        .ok_or_else(|| expected("error-workset-not-found"))?;

    workset_info.name = update.name.clone();

    workset_info.description = update.description.clone();

    workset_info.updated_at = now();

    Ok(())
}

impl<'a> Run<GetWorksetInfo<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &GetWorksetInfo<'a>,
    ) -> RegularResult<WorksetInfo> {
        //
        let state = self.state.lock().unwrap();

        get_workset_info(&state, oper.id)
    }
}

impl<'a> Run<ListWorksetInfos<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListWorksetInfos<'a>,
    ) -> RegularResult<Vec<WorksetInfo>> {
        //
        let state = self.state.lock().unwrap();

        Ok(list_workset_infos(&state, oper))
    }
}

impl<'a> Run<UpdateWorkset<'a>> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(&self, oper: &UpdateWorkset<'a>) -> RegularResult<()> {
        //
        let mut state = self.state.lock().unwrap();

        update_workset(&mut state, oper.update)
    }
}

impl<'a> Step<GetWorksetInfo<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetWorksetInfo<'a>,
    ) -> RegularResult<WorksetInfo> {
        get_workset_info(&context.state, oper.id)
    }
}

impl<'a> Step<ListWorksetInfos<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListWorksetInfos<'a>,
    ) -> RegularResult<Vec<WorksetInfo>> {
        Ok(list_workset_infos(&context.state, oper))
    }
}

impl<'a> Step<GetWorksetInfoExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &GetWorksetInfoExcluded<'a>,
    ) -> RegularResult<WorksetInfo> {
        get_workset_info(&context.state, oper.id)
    }
}

impl<'a> Step<ListWorksetInfosExcluded<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ListWorksetInfosExcluded<'a>,
    ) -> RegularResult<Vec<WorksetInfo>> {
        Ok(context
            .state
            .worksets
            .iter()
            .filter(|workset_info| workset_info.team_id == oper.team_id)
            .cloned()
            .collect())
    }
}

impl<'a> Step<CreateWorkset<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateWorkset<'a>,
    ) -> RegularResult<WorksetInfo> {
        //
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

        Ok(workset_info)
    }
}

impl<'a> Step<DeleteWorkset<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &DeleteWorkset<'a>,
    ) -> RegularResult<()> {
        //
        let position = context
            .state
            .worksets
            .iter()
            .position(|workset_info| workset_info.id == oper.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;

        let deleted_workset_id = context.state.worksets[position].id.clone();

        let deleted_comic_ids = context
            .state
            .comics
            .iter()
            .filter(|comic_info| comic_info.workset_id == deleted_workset_id)
            .map(|comic_info| comic_info.id.clone())
            .collect::<Vec<_>>();

        let deleted_chapter_ids = context
            .state
            .chapters
            .iter()
            .filter(|chapter_info| {
                deleted_comic_ids.contains(&chapter_info.comic_id)
            })
            .map(|chapter_info| chapter_info.id.clone())
            .collect::<Vec<_>>();

        context.state.worksets.remove(position);

        context
            .state
            .comics
            .retain(|comic_info| comic_info.workset_id != deleted_workset_id);

        context.state.chapters.retain(|chapter_info| {
            !deleted_comic_ids.contains(&chapter_info.comic_id)
        });

        context.state.pages.retain(|page_info| {
            !deleted_chapter_ids.contains(&page_info.chapter_id)
        });

        context.state.assignments.retain(|assignment_info| {
            !deleted_chapter_ids.contains(&assignment_info.chapter_id)
        });

        Ok(())
    }
}

impl<'a> Step<AllocWorksetComicIndex<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &AllocWorksetComicIndex<'a>,
    ) -> RegularResult<i32> {
        //
        // verify the workset exists
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
            .count() as i32;

        Ok(index)
    }
}

impl<'a> Step<UpdateWorksetComicCount<'a>, MockContext> for Mock {
    type Error = RegularError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &UpdateWorksetComicCount<'a>,
    ) -> RegularResult<()> {
        //
        let workset_info = context
            .state
            .worksets
            .iter_mut()
            .find(|workset_info| workset_info.id == oper.id)
            .ok_or_else(|| expected("error-workset-not-found"))?;

        workset_info.comic_count += oper.delta;

        workset_info.updated_at = now();

        Ok(())
    }
}
