//! Mock announcement repository operations.

use std::cmp::Reverse;

use poprako_orchestra::Run;
use tracing::instrument;

use crate::model::read::proj::announcement::AnnouncementInfo;
use crate::model::read::proj::user::UserInfo;
use crate::model::read::spec::announcement::AnnouncementListSpec;
use crate::model::write::announcement::{AnnouncementEntry, AnnouncementRepl};
use crate::part::repo::oper::announcement::{
    CreateAnnouncement, DeleteAnnouncement, GetAnnouncementInfo,
    ListAnnouncementInfos, UpdateAnnouncement,
};
use crate::part_impl::repo::mock_impl::{Mock, MockState, expected, now};
use crate::result::{BaseError, BaseRest, accept};
use crate::value::announcement::AnnouncementInclOpt;

// Internal implementation of `find_user`.
fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    //
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

// Internal implementation of `apply_user_incl`.
fn apply_user_incl(
    state: &MockState,
    announcement_info: &mut AnnouncementInfo,
    include_user: bool,
) {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    announcement_info.user = None;

    if include_user {
        announcement_info.user = find_user(state, &announcement_info.user_id);
    }
}

// Internal implementation of `list_announcements`.
fn list_announcements(
    state: &MockState,
    spec: &AnnouncementListSpec,
) -> Vec<AnnouncementInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    let include_user = spec.incl_opt.contains(&AnnouncementInclOpt::User);

    let mut announcement_infos = state
        .announcements
        .iter()
        .filter(|announcement_info| announcement_info.team_id == spec.team_id)
        .cloned()
        .collect::<Vec<_>>();

    announcement_infos
        .sort_by_key(|announcement_info| Reverse(announcement_info.created_at));

    for announcement_info in &mut announcement_infos {
        apply_user_incl(state, announcement_info, include_user);
    }

    let offset = spec.offset as usize;

    let limit = spec.limit as usize;

    match offset >= announcement_infos.len() {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        true => Vec::new(),

        false => {
            //
            // Internal implementation detail.
            // Internal implementation detail.
            let end = std::cmp::min(offset + limit, announcement_infos.len());

            announcement_infos[offset..end].to_vec()
        }
    }
}

// Internal implementation of `create_announcement`.
fn create_announcement(
    state: &mut MockState,
    entry: &AnnouncementEntry,
) -> BaseRest<AnnouncementInfo> {
    //
    // Internal implementation detail.
    // Internal implementation detail.
    if state
        .announcements
        .iter()
        .any(|announcement_info| announcement_info.id == entry.id)
    {
        return Err(expected("error-already-exists"));
    }

    let announcement_info = AnnouncementInfo {
        id: entry.id.clone(),
        team_id: entry.team_id.clone(),
        user_id: entry.user_id.clone(),
        user: None,
        title: entry.title.clone(),
        content: entry.content.clone(),
        created_at: now(),
    };

    state.announcements.push(announcement_info.clone());

    accept(announcement_info)
}

// Loads an announcement from mock storage for mutation.
fn get_announcement_info(
    state: &MockState,
    id: &str,
) -> BaseRest<AnnouncementInfo> {
    //
    state
        .announcements
        .iter()
        .find(|announcement_info| announcement_info.id == id)
        .cloned()
        .ok_or_else(|| expected("error-announcement-not-found"))
}

// Replaces an announcement's editable fields in mock storage.
fn update_announcement(
    state: &mut MockState,
    update: &AnnouncementRepl,
) -> BaseRest<()> {
    //
    let announcement_info = state
        .announcements
        .iter_mut()
        .find(|announcement_info| announcement_info.id == update.id)
        .ok_or_else(|| expected("error-announcement-not-found"))?;

    announcement_info.title = update.title.clone();

    announcement_info.content = update.content.clone();

    accept(())
}

// Deletes an announcement from mock storage.
fn delete_announcement(state: &mut MockState, id: &str) -> BaseRest<()> {
    //
    let announcement_index = state
        .announcements
        .iter()
        .position(|announcement_info| announcement_info.id == id)
        .ok_or_else(|| expected("error-announcement-not-found"))?;

    state.announcements.remove(announcement_index);

    accept(())
}

impl Run<ListAnnouncementInfos<'_>> for Mock {
    // Internal type alias for `Error`.
    // Defines the adapter error exposed by this operation.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &ListAnnouncementInfos<'_>,
    ) -> BaseRest<Vec<AnnouncementInfo>> {
        //
        // Internal implementation detail.
        // Internal implementation detail.
        let state = self.state.lock().unwrap();

        accept(list_announcements(&state, oper.spec))
    }
}

impl Run<CreateAnnouncement<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Internal implementation of `run`.
    async fn run(
        &self,
        oper: &CreateAnnouncement<'_>,
    ) -> BaseRest<AnnouncementInfo> {
        //
        let mut state = self.state.lock().unwrap();

        create_announcement(&mut state, oper.entry)
    }
}

impl Run<GetAnnouncementInfo<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Loads an announcement independently.
    async fn run(
        &self,
        oper: &GetAnnouncementInfo<'_>,
    ) -> BaseRest<AnnouncementInfo> {
        //
        let state = self.state.lock().unwrap();

        get_announcement_info(&state, oper.id)
    }
}

impl Run<UpdateAnnouncement<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Updates an announcement independently.
    async fn run(&self, oper: &UpdateAnnouncement<'_>) -> BaseRest<()> {
        //
        let mut state = self.state.lock().unwrap();

        update_announcement(&mut state, oper.update)
    }
}

impl Run<DeleteAnnouncement<'_>> for Mock {
    // Internal type alias for `Error`.
    type Error = BaseError;

    #[instrument(level = "info", skip_all)]
    // Deletes an announcement independently.
    async fn run(&self, oper: &DeleteAnnouncement<'_>) -> BaseRest<()> {
        //
        let mut state = self.state.lock().unwrap();

        delete_announcement(&mut state, oper.id)
    }
}
