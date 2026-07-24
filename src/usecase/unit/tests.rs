// list_infos(list_infos)(positive): team member lists page units in index order with page counters.
// list_infos(list_infos)(positive): chapter assignee lists page units without team membership.
// list_infos(list_infos)(negative): non-member without assignment cannot list page units.
// save_infos(save_infos)(positive): create maps a local id, save updates, and delete removes.
// save_infos(save_infos)(positive): successful translation and proofread submissions asynchronously start their chapter stages.
// save_infos(save_infos)(positive): save with before_id places unit before anchor, None appends to tail.
// save_infos(save_infos)(positive): concurrent merge applies b-then-c and c-then-b to twenty units and reaches consistent final state.
// save_infos(save_infos)(negative): user without edit role rolls back units and counters.
// save_infos(save_infos)(negative): invalid diff rolls back units, counters, and comic activity.
// save_infos(save_infos)(negative): missing text editor ids are rejected before transaction access.

use super::*;

use time::OffsetDateTime;

use crate::data::unit::UnitOperParams;
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::member::MemberInfo;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::{BaseError, ExpectedVariant, accept};
use crate::value::chapter::{Stage, StageMask, StagePhase};
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

mod basic;
mod merge;
mod rollback;

struct TestRng {
    state: u64,
}

impl TestRng {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next(&mut self) -> u64 {
        //
        self.state = self
            .state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);

        self.state
    }

    fn range(&mut self, bound: usize) -> usize {
        (self.next() as usize) % bound
    }

    fn bool(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn workset(id: &str, team_id: &str) -> WorksetInfo {
    //
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: team_id.into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        created_at: time,
        updated_at: time,
    }
}

fn comic(id: &str, workset_id: &str) -> ComicInfo {
    //
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: workset_id.into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        cover_hash: crate::value::image::ImageHash::default(),
        cover_ext: crate::value::image::ImageExt::Png,
        chapter_count: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(
    id: &str,
    comic_id: &str,
    total: i32,
    translated: i32,
    proofread: i32,
) -> ChapterInfo {
    //
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: id.into(),
        comic_id: comic_id.into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_count: 1,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn member(user_id: &str) -> MemberInfo {
    MemberInfo {
        id: format!("member-{}", user_id),
        user_id: user_id.into(),
        user_nickname: user_id.into(),
        user_last_active_at: OffsetDateTime::now_utc(),
        team_id: "team-1".into(),
        user: None,
        team: None,
        roles: RoleMask::from(RoleField::TRANSLATOR),
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
    //
    let time = OffsetDateTime::now_utc();

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

fn page(id: &str, total: i32, translated: i32, proofread: i32) -> PageInfo {
    //
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index: 0,
        image_key: None,
        image_uploaded: false,
        image_version: 0,
        image_hash: ImageHash::new([0u8; 32]),
        image_ext: ImageExt::Png,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        created_at: time,
        updated_at: time,
    }
}

fn unit(
    id: &str,
    page_id: &str,
    index: i32,
    text: &str,
    proofread_text: Option<&str>,
    proofread: bool,
) -> UnitInfo {
    //
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        index,
        is_bubble: true,
        is_proofread: proofread,
        x_coord: 1.0,
        y_coord: 2.0,
        translated_text: Some(text.into()),
        last_translator_id: None,
        proofread_text: proofread_text.map(Into::into),
        last_proofreader_id: None,
        created_at: time,
        updated_at: time,
    }
}

fn create_oper(
    local_id: &str,
    text: &str,
    before_id: Option<&str>,
) -> UnitOperParams {
    UnitOperParams::Create {
        local_id: local_id.into(),
        before_id: before_id.map(Into::into),
        is_bubble: true,
        is_proofread: false,
        x_coord: 3.0,
        y_coord: 4.0,
        translated_text: Some(text.into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
    }
}

fn save_oper(id: &str, text: &str, before_id: Option<&str>) -> UnitOperParams {
    save_oper_with_payload(id, text, false, 5.0, 6.0, before_id)
}

fn delete_oper(id: &str) -> UnitOperParams {
    UnitOperParams::Delete { id: id.into() }
}

fn save_oper_with_payload(
    id: &str,
    text: &str,
    proofread: bool,
    x_coord: f64,
    y_coord: f64,
    before_id: Option<&str>,
) -> UnitOperParams {
    UnitOperParams::Save {
        id: id.into(),
        before_id: before_id.map(Into::into),
        is_bubble: true,
        is_proofread: proofread,
        x_coord,
        y_coord,
        translated_text: Some(text.into()),
        last_translator_id: Some("user-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
    }
}

fn seed_scope(mock: &Mock, total: i32, translated: i32, proofread: i32) {
    //
    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_comic(comic("comic-1", "workset-1"));

    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        total,
        translated,
        proofread,
    ));

    mock.seed_page(page("page-1", total, translated, proofread));
}

fn sorted_unit_ids(units: &[UnitInfo]) -> Vec<String> {
    //
    let mut unit_infos = units.to_vec();

    unit_infos.sort_by_key(|left| left.index);

    unit_infos
        .into_iter()
        .map(|unit_info| unit_info.id)
        .collect()
}

async fn wait_for_stage(mock: &Mock, stage: Stage, phase: StagePhase) {
    //
    for _ in 0..100 {
        //
        if mock.snapshot().chapters[0].stages.has_phase(stage, phase) {
            return;
        }

        tokio::task::yield_now().await;
    }

    panic!("detached stage advancement did not finish");
}

fn assert_perm_error(error: BaseError) {
    match error {
        //
        BaseError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Perm));
        }

        BaseError::Unrecoverable { .. } => {
            panic!("expected permission error");
        }
    }
}

fn assert_args_error(error: BaseError) {
    match error {
        //
        BaseError::Expected { variant, .. } => {
            assert!(matches!(variant, ExpectedVariant::Args));
        }

        BaseError::Unrecoverable { .. } => {
            panic!("expected argument error");
        }
    }
}
