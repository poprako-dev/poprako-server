use super::super::alloc::alloc_chapter_page_sources;

use time::OffsetDateTime;

use crate::data::instr::page_source::{
    AllocChapterPageSourcesInstr, PageSourceImageInstr,
};
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::page_source::PageSourceInfo;
use crate::model::shared::user::UserToken;
use crate::part_impl::repo::mock_impl::Mock;
use crate::result::ExpectedVariant;
use crate::test_util::{IMAGE_CONFIG, assert_expected_variant};
use crate::value::chapter::mask::StageMask;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

fn token() -> UserToken {
    UserToken {
        user_id: "user--00000000001".into(),
    }
}

fn comic() -> ComicInfo {
    let time = OffsetDateTime::now_utc();

    ComicInfo {
        id: "comic--00000000001".into(),
        workset_id: "workset--00000000001".into(),
        index: 0,
        title: "comic".into(),
        author: "author".into(),
        description: None,
        chapter_count: 1,
        creator_id: "user--00000000001".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        archived_at: None,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(page_source_count: usize) -> ChapterInfo {
    let time = OffsetDateTime::now_utc();

    ChapterInfo {
        confirmed_artwork_ver: None,
        id: "chapter--00000000001".into(),
        comic_id: "comic--00000000001".into(),
        comic: None,
        is_pinned: true,
        index: 0,
        subtitle: "chapter".into(),
        page_source_count,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        stages: StageMask::try_from(0u32).unwrap(),
        creator_id: "user--00000000001".into(),
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment() -> AssignmentInfo {
    let time = OffsetDateTime::now_utc();

    AssignmentInfo {
        id: "assignment--00000000001".into(),
        chapter_id: "chapter--00000000001".into(),
        user_id: "user--00000000001".into(),
        user: None,
        chapter: None,
        roles: RoleMask::from(RoleField::RAW_PROVIDER),
        created_at: time,
        updated_at: time,
    }
}

fn page(
    id: &str,
    index: usize,
    total: usize,
    translated: usize,
    proofread: usize,
) -> PageSourceInfo {
    let time = OffsetDateTime::now_utc();

    PageSourceInfo {
        id: id.into(),
        chapter_id: "chapter--00000000001".into(),
        index,
        total_unit_count: total,
        translated_unit_count: translated,
        proofread_unit_count: proofread,
        created_at: time,
        updated_at: time,
    }
}

fn page_source_instr(
    page_source_id: Option<&str>,
    hash: u8,
    new_byte_len: Option<u64>,
) -> PageSourceImageInstr {
    PageSourceImageInstr {
        raw_ident: None,
        page_source_id: page_source_id.map(Into::into),
        image_hash: ImageHash::new([hash; 32]),
        new_byte_len,
        ext: ImageExt::Png,
    }
}

fn seed_scope(mock: &Mock, page_source_count: usize) {
    mock.seed_comic(comic());

    mock.seed_chapter(chapter(page_source_count));

    mock.seed_assignment(assignment());
}

#[tokio::test]
async fn mixed_manifest_preserves_order_counts_and_object_obligations() {
    let mock = Mock::new();

    seed_scope(&mock, 3);

    mock.seed_page(page("page_source--34844962250", 0, 3, 2, 1));

    mock.seed_page(page("page_source--35727087946", 1, 4, 3, 2));

    mock.seed_page(page("page_source--89610989726", 2, 9, 8, 7));

    mock.seed_page_source_image_obj("page_source--34844962250", "png");

    mock.seed_page_source_image_obj("page_source--35727087946", "png");

    mock.seed_page_source_image_obj("page_source--89610989726", "png");

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages: vec![
            page_source_instr(Some("page_source--35727087946"), 0, None),
            page_source_instr(None, 5, Some(1024)),
            page_source_instr(Some("page_source--34844962250"), 6, Some(2048)),
        ],
    };

    let reserved = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .unwrap();

    assert_eq!(reserved.pages.len(), 3);

    assert_eq!(
        reserved.pages.first().unwrap().page_source_id,
        "page_source--35727087946"
    );

    assert_eq!(reserved.pages.first().unwrap().index, 0);

    assert!(reserved.pages.first().unwrap().slot.is_none());

    assert_eq!(reserved.pages.get(1).unwrap().index, 1);

    assert!(reserved.pages.get(1).unwrap().slot.is_some());

    assert_eq!(
        reserved.pages.get(2).unwrap().page_source_id,
        "page_source--34844962250"
    );

    assert_eq!(reserved.pages.get(2).unwrap().index, 2);

    assert!(reserved.pages.get(2).unwrap().slot.is_some());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 3);

    assert!(
        snapshot
            .pages
            .iter()
            .all(|page| page.id != "page_source--89610989726")
    );

    let retained_page = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page_source--35727087946")
        .unwrap();

    assert_eq!(retained_page.total_unit_count, 4);

    assert_eq!(retained_page.translated_unit_count, 3);

    assert_eq!(retained_page.proofread_unit_count, 2);

    let chapter = snapshot
        .chapters
        .iter()
        .find(|chapter| chapter.id == "chapter--00000000001")
        .unwrap();

    assert_eq!(chapter.page_source_count, 3);

    assert_eq!(chapter.total_unit_count, 7);

    assert_eq!(chapter.translated_unit_count, 5);

    assert_eq!(chapter.proofread_unit_count, 3);

    assert_eq!(snapshot.prom_records.len(), 1);

    assert_eq!(snapshot.obj_tasks.len(), 4);
}

#[tokio::test]
async fn maximum_manifest_reserves_all_page_sources_in_input_order() {
    let mock = Mock::new();

    seed_scope(&mock, 0);

    let pages = (0u8..200)
        .map(|index| page_source_instr(None, index, Some(1024)))
        .collect::<Vec<_>>();

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages,
    };

    let reserved = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .unwrap();

    assert_eq!(reserved.pages.len(), 200);

    assert!(reserved.pages.iter().enumerate().all(|(index, page)| {
        usize::try_from(page.index).ok() == Some(index) && page.slot.is_some()
    }));

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 200);

    assert_eq!(snapshot.obj_tasks.len(), 200);

    assert_eq!(snapshot.prom_records.len(), 1);
}

#[tokio::test]
async fn unknown_retained_page_source_rejects_before_manifest_writes() {
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page_source--34844962250", 0, 3, 2, 1));

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages: vec![page_source_instr(Some("unknown-page"), 0, None)],
    };

    let result = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await;

    assert!(result.is_err());

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(
        snapshot.pages.first().unwrap().id,
        "page_source--34844962250"
    );

    assert_eq!(snapshot.pages.first().unwrap().index, 0);

    assert_eq!(snapshot.chapters.first().unwrap().page_source_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn missing_retained_object_rolls_back_manifest_reorder() {
    let mock = Mock::new();

    seed_scope(&mock, 2);

    mock.seed_page(page("page_source--34844962250", 0, 3, 2, 1));

    mock.seed_page(page("page_source--35727087946", 1, 4, 3, 2));

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages: vec![
            page_source_instr(Some("page_source--35727087946"), 0, None),
            page_source_instr(Some("page_source--34844962250"), 0, None),
        ],
    };

    let result = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await;

    assert!(result.is_err());

    let snapshot = mock.snapshot();

    let page_source_a = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page_source--34844962250")
        .unwrap();

    let page_source_b = snapshot
        .pages
        .iter()
        .find(|page| page.id == "page_source--35727087946")
        .unwrap();

    assert_eq!(page_source_a.index, 0);

    assert_eq!(page_source_b.index, 1);

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn duplicate_page_source_id_is_args_error_without_side_effects() {
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page_source--34844962250", 0, 3, 2, 1));

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages: vec![
            page_source_instr(Some("page_source--34844962250"), 1, Some(1024)),
            page_source_instr(Some("page_source--34844962250"), 2, Some(2048)),
        ],
    };

    let error = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 1);

    assert_eq!(
        snapshot.pages.first().unwrap().id,
        "page_source--34844962250"
    );

    assert_eq!(snapshot.pages.first().unwrap().index, 0);

    assert_eq!(snapshot.pages.first().unwrap().total_unit_count, 3);

    assert_eq!(snapshot.pages.first().unwrap().translated_unit_count, 2);

    assert_eq!(snapshot.pages.first().unwrap().proofread_unit_count, 1);

    assert_eq!(snapshot.chapters.first().unwrap().page_source_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}

#[tokio::test]
async fn cross_chapter_retained_page_source_is_args_error_without_side_effects()
{
    let mock = Mock::new();

    seed_scope(&mock, 1);

    mock.seed_page(page("page_source--34844962250", 0, 3, 2, 1));

    let mut other_chapter = chapter(1);

    other_chapter.id = "chapter--00000000002".into();

    mock.seed_chapter(other_chapter);

    let mut other_page = page("page_source--85117610840", 0, 5, 4, 3);

    other_page.chapter_id = "chapter--00000000002".into();

    mock.seed_page(other_page);

    let instr = AllocChapterPageSourcesInstr {
        chapter_id: "chapter--00000000001".into(),
        pages: vec![page_source_instr(
            Some("page_source--85117610840"),
            0,
            None,
        )],
    };

    let error = alloc_chapter_page_sources(
        (&mock, &mock, &mock, &mock, &IMAGE_CONFIG),
        token(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.pages.len(), 2);

    assert_eq!(
        snapshot.pages.first().unwrap().id,
        "page_source--34844962250"
    );

    assert_eq!(snapshot.pages.first().unwrap().index, 0);

    assert_eq!(
        snapshot.pages.get(1).unwrap().id,
        "page_source--85117610840"
    );

    assert_eq!(snapshot.pages.get(1).unwrap().index, 0);

    assert_eq!(snapshot.pages.get(1).unwrap().total_unit_count, 5);

    assert_eq!(snapshot.pages.get(1).unwrap().translated_unit_count, 4);

    assert_eq!(snapshot.pages.get(1).unwrap().proofread_unit_count, 3);

    assert_eq!(snapshot.chapters.first().unwrap().page_source_count, 1);

    assert_eq!(snapshot.chapters.get(1).unwrap().page_source_count, 1);

    assert!(snapshot.objs.is_empty());

    assert!(snapshot.obj_tasks.is_empty());

    assert!(snapshot.prom_records.is_empty());
}
