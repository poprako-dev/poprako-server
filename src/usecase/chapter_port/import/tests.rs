// import(import)(positive): proofreader imports real LabelPlus material transactionally.
// import(import)(negative): page-count mismatch rejects import and leaves units and counters unchanged.

use super::*;

use time::OffsetDateTime;

use crate::data::chapter_port::ChapterTranslationImportData;
use crate::model::assignment::AssignmentInfo;
use crate::model::chapter::ChapterInfo;
use crate::model::comic::ComicInfo;
use crate::model::page::PageInfo;
use crate::model::unit::UnitInfo;
use crate::model::user::UserToken;
use crate::model::workset::WorksetInfo;
use crate::part_impl::repo_mock::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::assert_expected_variant;
use crate::value::chapter::StageMask;
use crate::value::chapter_port::TranslationFormat;
use crate::value::role::{RoleField, RoleMask};

const LABEL_PLUS_MATERIAL: &str =
    include_str!("../../../../tests/materials/translations.lp.txt");

fn token(user_id: &str) -> UserToken {
    UserToken {
        user_id: user_id.into(),
    }
}

fn comic(id: &str) -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: id.into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        is_completed: false,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str) -> WorksetInfo {
    let time = OffsetDateTime::now_utc();

    WorksetInfo {
        id: id.into(),
        team_id: "team-1".into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        comic_next_index: 1,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(
    page_count: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        id: "chapter-1".into(),
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> AssignmentInfo {
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

fn page(
    id: &str,
    index: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) -> PageInfo {
    let time = OffsetDateTime::now_utc();

    PageInfo {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: Some(format!("page-{}.png", index)),
        image_uploaded: true,
        image_version: 1,
        total_unit_count,
        translated_unit_count: total_unit_count,
        proofread_unit_count,
        created_at: time,
        updated_at: time,
    }
}

fn unit(id: &str, page_id: &str, index: i32, text: &str) -> UnitInfo {
    let time = OffsetDateTime::now_utc();

    UnitInfo {
        id: id.into(),
        page_id: page_id.into(),
        index,
        is_bubble: true,
        is_proofread: false,
        x_coord: 0.25,
        y_coord: 0.75,
        translated_text: Some(text.into()),
        last_translator_id: Some("translator-1".into()),
        proofread_text: None,
        last_proofreader_id: None,
        created_at: time,
        updated_at: time,
    }
}

fn seed_base(
    mock: &Mock,
    page_count: i32,
    total_unit_count: i32,
    proofread_unit_count: i32,
) {
    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter(
        page_count,
        total_unit_count,
        proofread_unit_count,
    ));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));
}

fn seed_material_pages(mock: &Mock) {
    for index in 0..9 {
        mock.seed_page(page(&format!("page-{}", index + 1), index, 0, 0));
    }
}

#[tokio::test]
async fn import_label_plus_material_updates_units_and_counters() {
    let mock = Mock::new();

    seed_base(&mock, 9, 0, 0);

    seed_material_pages(&mock);

    let imported = import(
        &mock,
        &mock,
        token("user-1"),
        ChapterTranslationImportData {
            format: TranslationFormat::LabelPlus,
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await;

    let imported = match imported {
        Ok(imported) => imported,
        Err(_) => panic!("expected import success"),
    };

    let snapshot = mock.snapshot();

    let first_unit = snapshot
        .units
        .iter()
        .find(|unit_info| unit_info.page_id == "page-1" && unit_info.index == 0)
        .unwrap();

    let last_unit = snapshot
        .units
        .iter()
        .find(|unit_info| unit_info.page_id == "page-9" && unit_info.index == 8)
        .unwrap();

    let chapter_info = snapshot
        .chapters
        .iter()
        .find(|chapter_info| chapter_info.id == "chapter-1")
        .unwrap();

    assert_eq!(imported.imported_page_count, 9);

    assert_eq!(imported.imported_unit_count, 65);

    assert_eq!(snapshot.units.len(), 65);

    assert_eq!(first_unit.proofread_text, Some("喂 游斗哥".into()));

    assert_eq!(first_unit.last_proofreader_id, Some("user-1".into()));

    assert_eq!(
        last_unit.proofread_text,
        Some("哥哥对次女可爱的\n小心思毫无察觉".into())
    );

    assert_eq!(chapter_info.total_unit_count, 65);

    assert_eq!(chapter_info.proofread_unit_count, 65);
}

#[tokio::test]
async fn import_rejects_page_count_mismatch_without_mutation() {
    let mock = Mock::new();

    seed_base(&mock, 2, 1, 0);

    mock.seed_page(page("page-1", 0, 1, 0));

    mock.seed_page(page("page-2", 1, 0, 0));

    mock.seed_unit(unit("unit-a", "page-1", 0, "old"));

    let err = import(
        &mock,
        &mock,
        token("user-1"),
        ChapterTranslationImportData {
            format: TranslationFormat::LabelPlus,
            content: LABEL_PLUS_MATERIAL.into(),
        },
        "chapter-1".into(),
    )
    .await
    .err()
    .unwrap();

    let snapshot = mock.snapshot();

    assert_expected_variant(err, ExpectedVariant::Args);

    assert_eq!(snapshot.units.len(), 1);

    assert_eq!(snapshot.units[0].translated_text, Some("old".into()));

    assert_eq!(snapshot.chapters[0].total_unit_count, 1);
}
