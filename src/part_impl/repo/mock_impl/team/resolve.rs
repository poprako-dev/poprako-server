//! In-memory team ownership projections.

#[cfg(test)]
mod tests;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::part::nucl::ReptRead;
use crate::part::repo::oper::team::ResolveTeamId;
use crate::part_impl::repo::mock_impl::{
    Mock, MockContext, MockState, expected, unrecoverable,
};
use crate::result::{BaseError, BaseRest, accept};

// Resolve the owning team for a comic from in-memory state.
fn resolve_comic_team_id(state: &MockState, id: &str) -> BaseRest<String> {
    //
    let comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == id)
        .ok_or_else(|| expected("error-comic-not-found"))?;

    let workset_info = state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .ok_or_else(|| {
            //
            unrecoverable(
                "[resolve_comic_team_id] comic references missing workset",
            )
        })?;

    accept(workset_info.team_id.clone())
}

// Resolve the owning team for a chapter from in-memory state.
fn resolve_chapter_team_id(state: &MockState, id: &str) -> BaseRest<String> {
    //
    let chapter_info = state
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == id)
        .ok_or_else(|| expected("error-chapter-not-found"))?;

    let comic_info = state
        .comics
        .iter()
        .find(|comic_info| comic_info.id == chapter_info.comic_id)
        .ok_or_else(|| {
            //
            unrecoverable(
                "[resolve_chapter_team_id] chapter references missing comic",
            )
        })?;

    let workset_info = state
        .worksets
        .iter()
        .find(|workset_info| workset_info.id == comic_info.workset_id)
        .ok_or_else(|| {
            //
            unrecoverable(
                "[resolve_chapter_team_id] comic references missing workset",
            )
        })?;

    accept(workset_info.team_id.clone())
}

// Dispatch a team-ownership query to the comic or chapter resolver.
fn resolve_team_id(
    state: &MockState,
    oper: &ResolveTeamId<'_>,
) -> BaseRest<String> {
    //
    match oper {
        //
        ResolveTeamId::Comic { id } => resolve_comic_team_id(state, id),

        ResolveTeamId::Chapter { id } => resolve_chapter_team_id(state, id),
    }
}

impl Run<ResolveTeamId<'_>> for Mock {
    // BaseError for the standalone mock team-resolution projection.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Runs the team-resolution projection against the shared mock state.
    #[instrument(level = "info", skip_all)]
    async fn run(
        &self,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        //
        let state = self.state.lock().unwrap();

        resolve_team_id(&state, oper)
    }
}

impl Step<ResolveTeamId<'_>, MockContext> for Mock {
    // BaseError for the transactional mock team-resolution projection.
    type Level = ReptRead;

    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    // Runs the team-resolution projection inside the transactional mock context.
    #[instrument(level = "info", skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &ResolveTeamId<'_>,
    ) -> Result<String, Self::Error> {
        resolve_team_id(&context.state, oper)
    }
}
