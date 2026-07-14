use super::*;

use crate::data::assignment::{
    ListAssignmentInfosParams, UpdateAssignmentRolesParams,
};
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::team::TeamInfo;
use crate::model::user::{UserCredential, UserInfo, UserToken};
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::test_util::now;
use crate::value::chapter::StageMask;
use crate::value::role::{RoleField, RoleMask};

mod delete;
mod join;
mod list_infos;
mod update_roles;

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn role(role_field: RoleField) -> RoleMask {
    RoleMask::from(role_field)
}

fn roles(left: RoleField, right: RoleField) -> RoleMask {
    role(left).union(role(right))
}

fn user(id: &str, is_sadmin: bool) -> UserInfo {
    //
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: id.into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        is_sadmin,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn credential(user_id: &str) -> UserCredential {
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

fn team(id: &str) -> TeamInfo {
    //
    let time = now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        avatar_key: None,
        avatar_uploaded: false,
        avatar_version: 0,
        workset_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = now();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: id.into(),
        description: None,
        comic_count: 0,
        comic_next_index: 0,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    let time = now();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: id.into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
        creator_id: "creator-user".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str, comic_id: &str) -> ChapterInfo {
    //
    let time = now();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "subtitle".into(),
        page_count: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "creator-user".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str, role_mask: RoleMask) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: now(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: role_mask,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    let time = now();

    AssignmentInfo {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn page(id: &str, chapter_id: &str, image_key: &str) -> PageInfo {
    //
    let time = now();

    PageInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        index: 0,
        image_key: Some(image_key.into()),
        image_uploaded: true,
        image_version: 1,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn list_by_chapter_data(chapter_id: &str) -> ListAssignmentInfosParams {
    ListAssignmentInfosParams {
        incl_opt: Vec::new(),
        chapter_id: Some(chapter_id.into()),
        owner_id: None,
        role: None,
        offset: 0,
        limit: 10,
    }
}

fn list_by_user_data(owner_id: &str) -> ListAssignmentInfosParams {
    ListAssignmentInfosParams {
        incl_opt: Vec::new(),
        chapter_id: None,
        owner_id: Some(owner_id.into()),
        role: None,
        offset: 0,
        limit: 10,
    }
}

fn update_roles_data(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> UpdateAssignmentRolesParams {
    UpdateAssignmentRolesParams {
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        roles: role_mask,
    }
}

fn seed_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1"));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_chapter(chapter("chapter-1", "comic-1"));
}

fn seed_user(mock: &Mock, user_id: &str, is_sadmin: bool) {
    mock.seed_user(user(user_id, is_sadmin), credential(user_id));
}
