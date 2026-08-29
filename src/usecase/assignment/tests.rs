// Tests for assignment deletion behavior.
mod delete;
// Tests for assignment joining behavior.
mod join;
// Tests for assignment listing behavior.
mod list_infos;
// Tests for assignment role mutation behavior.
mod update_roles;

use super::*;

use crate::data::instr::assignment::{
    ListAssignmentInfosInstr, UpdateAssignmentRolesInstr,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::member::MemberInfo;
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::team::TeamInfo;
use crate::model::read::proj::user::{UserCredential, UserInfo};
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::test_util::now;
use crate::usecase::assignment::update_roles::update_roles;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::role::{RoleField, RoleMask};

// Build a token fixture for authenticated user_id.
fn token(user_id: &str) -> UserToken {
    // Build an auth token fixture for assignment scenario checks.
    UserToken {
        user_id: user_id.into(),
    }
}

// Build a one-bit role mask from a named role.
fn role(role_field: RoleField) -> RoleMask {
    // Build a single role mask flag.
    RoleMask::from(role_field)
}

// Combine two role fields for assignment fixture inputs.
fn roles(left: RoleField, right: RoleField) -> RoleMask {
    // Combine two role flags for assignment test inputs.
    role(left).union(role(right))
}

// Build a baseline user fixture for assignment tests.
fn user(id: &str, is_sadmin: bool) -> UserInfo {
    //
    // Build a member fixture with deterministic timestamps.
    let time = now();

    UserInfo {
        id: id.into(),
        qid: id.into(),
        nickname: id.into(),
        is_sadmin,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

// Build a fixed credential fixture for auth-dependent assertions.
fn credential(user_id: &str) -> UserCredential {
    // Build a deterministic credential for authentication preconditions.
    UserCredential {
        user_id: user_id.into(),
        password_hash: "hash".into(),
    }
}

// Build a minimal team fixture used by chapter and workset ownership.
fn team(id: &str) -> TeamInfo {
    //
    // Build a team stub used by assignment ownership assertions.
    let time = now();

    TeamInfo {
        id: id.into(),
        name: id.into(),
        description: "description".into(),
        created_at: time,
        updated_at: time,
    }
}

// Build a workset fixture binding team + identifier inputs.
fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    // Build a workset fixture for assignment scoping checks.
    let time = now();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: id.into(),
        description: None,
        comic_count: 0,
        created_at: time,
        updated_at: time,
    }
}

// Build a comic fixture for assignment domain test data.
fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    // Build a comic fixture under a target workset.
    let time = now();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: id.into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,
        creator_id: "creator-user".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

// Build a chapter fixture linked to the selected comic id.
fn chapter(id: &str, comic_id: &str) -> ChapterInfo {
    //
    // Build a chapter fixture for assignment operations.
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

// Build a member fixture with explicit role mask for membership assertions.
fn member(user_id: &str, role_mask: RoleMask) -> MemberInfo {
    // Build a team member fixture with a deterministic role set.
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

// Build an assignment fixture for a user/chapter role setup.
fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    // Build an assignment row fixture for role and membership scenarios.
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

// Build a page fixture with stable image metadata.
fn page(id: &str, chapter_id: &str, _image_key: &str) -> PageInfo {
    //
    // Build a page fixture with image metadata for assignment page listings.
    let time = now();

    PageInfo {
        id: id.into(),
        chapter_id: chapter_id.into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

// Build list instr that filter by chapter id.
fn list_by_chapter_data(chapter_id: &str) -> ListAssignmentInfosInstr {
    ListAssignmentInfosInstr {
        incl_opt: Vec::new(),
        chapter_id: Some(chapter_id.into()),
        owner_id: None,
        role: None,
        offset: 0,
        limit: 10,
    }
}

// Build list instr that filter by owner id.
fn list_by_user_data(owner_id: &str) -> ListAssignmentInfosInstr {
    ListAssignmentInfosInstr {
        incl_opt: Vec::new(),
        chapter_id: None,
        owner_id: Some(owner_id.into()),
        role: None,
        offset: 0,
        limit: 10,
    }
}

// Build role-update instr for assignment mutability tests.
fn update_roles_data(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> UpdateAssignmentRolesInstr {
    UpdateAssignmentRolesInstr {
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        roles: role_mask,
    }
}

// Seed shared team/workset/comic/chapter fixtures.
fn seed_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1"));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_chapter(chapter("chapter-1", "comic-1"));
}

// Seed a chapter that is frozen because publishing has completed.
fn seed_published_scope(mock: &Mock) {
    //
    mock.seed_team(team("team-1"));

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    let mut chapter_info = chapter("chapter-1", "comic-1");

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Publish, StagePhase::Completed)
        .unwrap();

    mock.seed_chapter(chapter_info);
}

// Seed a user and credential for repeatable test preparation.
fn seed_user(mock: &Mock, user_id: &str, is_sadmin: bool) {
    mock.seed_user(user(user_id, is_sadmin), credential(user_id));
}
