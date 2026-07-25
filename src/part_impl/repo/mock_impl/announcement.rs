//! Mock announcement repository operations.

use std::cmp::Reverse;

use poprako_orchestra::{Run, Step};
use tracing::instrument;

use crate::model::announcement::{AnnouncementEntry, AnnouncementInfo, AnnouncementListSpec};
use crate::model::user::UserInfo;
use crate::part::repo::oper::announcement::{CreateAnnouncement, ListAnnouncementInfos};
use crate::part_impl::repo::mock_impl::{Mock, MockContext, MockState, expected, now};
use crate::result::{BaseError, BaseResult, accept};
use crate::value::announcement::AnnouncementInclOpt;

fn find_user(state: &MockState, user_id: &str) -> Option<UserInfo> {
    state
        .users
        .iter()
        .find(|user_info| user_info.id == user_id)
        .cloned()
}

fn apply_user_incl(
    state: &MockState,
    announcement_info: &mut AnnouncementInfo,
    include_user: bool,
) {
    //
    announcement_info.user = None;

    if include_user {
        announcement_info.user = find_user(state, &announcement_info.user_id);
    }
}

fn list_announcements(
    state: &MockState,
    spec: &AnnouncementListSpec,
) -> Vec<AnnouncementInfo> {
    //
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
        true => Vec::new(),

        false => {
            //
            let end = std::cmp::min(offset + limit, announcement_infos.len());

            announcement_infos[offset..end].to_vec()
        }
    }
}

fn create_announcement(
    state: &mut MockState,
    entry: &AnnouncementEntry,
) -> BaseResult<AnnouncementInfo> {
    //
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

impl Run<ListAnnouncementInfos<'_>> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn run(
        &self,
        oper: &ListAnnouncementInfos<'_>,
    ) -> BaseResult<Vec<AnnouncementInfo>> {
        //
        let state = self.state.lock().unwrap();

        accept(list_announcements(&state, oper.spec))
    }
}

impl Step<CreateAnnouncement<'_>, MockContext> for Mock {
    type Error = BaseError;

    #[instrument(level = "info", err(Debug), skip_all)]
    async fn step(
        &self,
        context: &mut MockContext,
        oper: &CreateAnnouncement<'_>,
    ) -> BaseResult<AnnouncementInfo> {
        create_announcement(&mut context.state, oper.entry)
    }
}
