//! Benchmark-only access to CPU-intensive application operations.

use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

use time::OffsetDateTime;

use crate::complex::chapter_port::{
    ChapterExportComplex, ChapterImportComplex,
};
use crate::complex::comic_archive::ComicArchiveComplex;
use crate::model::read::proj::assignment::AssignmentInfo;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::read::proj::comic_archive::{
    ComicArchiveChapterSnapshot, ComicArchivePageSnapshot, ComicArchiveSnapshot,
};
use crate::model::read::proj::page::PageInfo;
use crate::model::read::proj::unit::{UnitInfo, UnitOrder};
use crate::model::read::proj::user::UserInfo;
use crate::model::read::proj::workset::WorksetInfo;
use crate::model::shared::unit::UnitCoord;
use crate::value::chapter::StageMask;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::role::{RoleField, RoleMask};

pub use crate::complex::user::UserComplex;

// Number of chapters generated in the synthetic benchmark archive payload.
const CHAPTER_COUNT: usize = 8;

// Number of pages included in each benchmark chapter snapshot.
const PAGE_COUNT: usize = 8;

// Number of units included on each benchmark page.
const UNIT_COUNT: usize = 48;

/// Benchmarks archive preparation for a large comic snapshot.
pub struct ArchiveInput(ComicArchiveSnapshot);

/// Builds one large comic-archive input outside the benchmark measurement.
pub fn archive_input() -> Option<ArchiveInput> {
    archive_snapshot().map(ArchiveInput)
}

/// Runs archive preparation for a pre-built large comic snapshot.
pub async fn prepare_archive(archive_input: ArchiveInput) -> bool {
    //
    let ArchiveInput(comic_archive_snapshot) = archive_input;

    ComicArchiveComplex::prepare_entry(
        comic_archive_snapshot,
        "benchmark-user".into(),
        OffsetDateTime::UNIX_EPOCH,
    )
    .await
    .is_ok()
}

/// Benchmarks LabelPlus parsing with a repeated real-world import payload.
pub fn parse_label_plus() -> bool {
    ChapterImportComplex::parse_label_plus(label_plus_content()).is_ok()
}

/// Benchmarks PopRaKo JSON parsing with a large generated project payload.
pub fn parse_poprako() -> bool {
    ChapterImportComplex::parse_poprako(poprako_content()).is_ok()
}

/// Benchmarks LabelPlus rendering for a large page-and-unit collection.
pub struct LabelPlusExportInput {
    //
    /// Pages of the exported chapter used in the benchmark.
    pages: Vec<PageInfo>,
    /// Translation units keyed by their parent page ID.
    units_by_page_id: HashMap<String, Vec<UnitInfo>>,
}

/// Builds one large LabelPlus export input outside the benchmark measurement.
pub fn label_plus_export_input() -> LabelPlusExportInput {
    export_input()
}

/// Renders LabelPlus output for a pre-built page-and-unit collection.
pub fn make_label_plus(label_plus_export_input: &LabelPlusExportInput) -> bool {
    //
    !ChapterExportComplex::make_label_plus(
        &label_plus_export_input.pages,
        &label_plus_export_input.units_by_page_id,
    )
    .is_empty()
}

/// Benchmarks linked-list reconstruction over one maximum-size Page.
pub struct UnitOrderInput(Vec<UnitOrder>);

/// Builds an unordered maximum-size Unit chain.
pub fn unit_order_input() -> UnitOrderInput {
    //
    UnitOrderInput(
        (0..100)
            .map(|index| UnitOrder {
                id: format!("unit-{}", index),
                next_id: (index < 99).then(|| format!("unit-{}", index + 1)),
                is_hidden: false,
            })
            .rev()
            .collect(),
    )
}

/// Reconstructs visible IDs from a pre-built linked list.
pub fn order_visible_unit_ids(unit_order_input: UnitOrderInput) -> bool {
    //
    let mut orders_by_id = unit_order_input
        .0
        .into_iter()
        .map(|unit_order| (unit_order.id.clone(), unit_order))
        .collect::<HashMap<_, _>>();

    let successor_ids = orders_by_id
        .values()
        .filter_map(|unit_order| unit_order.next_id.as_ref())
        .collect::<HashSet<_>>();

    let head_ids = orders_by_id
        .keys()
        .filter(|id| !successor_ids.contains(id))
        .cloned()
        .collect::<Vec<_>>();

    let [head_id] = head_ids.as_slice() else {
        return false;
    };

    let mut visible_count = 0;

    let mut current_id = Some(head_id.clone());

    while let Some(id) = current_id {
        //
        let Some(unit_order) = orders_by_id.remove(&id) else {
            return false;
        };

        if !unit_order.is_hidden {
            visible_count += 1;
        }

        current_id = unit_order.next_id;
    }

    visible_count == 100 && orders_by_id.is_empty()
}

// Builds one large benchmark archive snapshot with deterministic content.
fn archive_snapshot() -> Option<ComicArchiveSnapshot> {
    //
    let stages = StageMask::try_from(0).ok()?;

    let archived_at = OffsetDateTime::UNIX_EPOCH;

    let mut chapter_snapshots = Vec::with_capacity(CHAPTER_COUNT);

    for chapter_index in 0..CHAPTER_COUNT {
        //
        let chapter_id = format!("chapter-{}", chapter_index);

        let user_info = user_info(archived_at);

        let assignment_info = AssignmentInfo {
            id: format!("assignment-{}", chapter_index),
            chapter_id: chapter_id.clone(),
            user_id: "user-1".into(),
            user: Some(user_info),
            chapter: None,
            roles: RoleMask::from(RoleField::TRANSLATOR),
            created_at: archived_at,
            updated_at: archived_at,
        };

        let mut page_snapshots = Vec::with_capacity(PAGE_COUNT);

        for page_index in 0..PAGE_COUNT {
            //
            let page_id = format!("page-{}-{}", chapter_index, page_index);

            let mut unit_infos = Vec::with_capacity(UNIT_COUNT);

            for unit_index in 0..UNIT_COUNT {
                //
                unit_infos.push(unit_info(
                    &page_id,
                    chapter_index,
                    page_index,
                    unit_index,
                    archived_at,
                ));
            }

            page_snapshots.push(ComicArchivePageSnapshot {
                page_info: PageInfo {
                    id: page_id,
                    chapter_id: chapter_id.clone(),
                    index: page_index as i32,
                    image_key: Some(format!(
                        "pages/{}-{}.webp",
                        chapter_index, page_index,
                    )),
                    is_image_uploaded: Some(true),
                    image_version: Some(1),
                    image_hash: Some(ImageHash::new([0u8; 32])),
                    image_ext: Some(ImageExt::Webp),
                    total_unit_count: UNIT_COUNT as i32,
                    translated_unit_count: UNIT_COUNT as i32,
                    proofread_unit_count: UNIT_COUNT as i32,
                    created_at: archived_at,
                    updated_at: archived_at,
                },
                unit_infos,
            });
        }

        chapter_snapshots.push(ComicArchiveChapterSnapshot {
            chapter_info: ChapterInfo {
                id: chapter_id,
                comic_id: "comic-1".into(),
                comic: None,
                is_pinned: chapter_index == 0,
                index: chapter_index as i32,
                subtitle: format!("Chapter {}", chapter_index),
                page_count: PAGE_COUNT as i32,
                total_unit_count: (PAGE_COUNT * UNIT_COUNT) as i32,
                translated_unit_count: (PAGE_COUNT * UNIT_COUNT) as i32,
                proofread_unit_count: (PAGE_COUNT * UNIT_COUNT) as i32,
                stages,
                creator_id: "user-1".into(),
                creator: None,
                created_at: archived_at,
                updated_at: archived_at,
            },
            assignment_infos: vec![assignment_info],
            workflow_record_infos: Vec::new(),
            page_snapshots,
        });
    }

    Some(ComicArchiveSnapshot {
        comic_info: ComicInfo {
            id: "comic-1".into(),
            workset_id: "workset-1".into(),
            index: 0,
            title: "Benchmark Comic".into(),
            author: "Benchmark Author".into(),
            description: Some("Benchmark archive payload".into()),
            cover_key: Some("covers/comic-1.webp".into()),
            is_cover_uploaded: Some(true),
            cover_version: Some(1),
            cover_hash: Some(ImageHash::default()),
            cover_ext: Some(ImageExt::Webp),
            chapter_count: CHAPTER_COUNT as i32,
            creator_id: "user-1".into(),
            workset: None,
            team: None,
            creator: None,
            last_active_at: archived_at,
            archived_at: None,
            created_at: archived_at,
            updated_at: archived_at,
        },
        workset_info: WorksetInfo {
            id: "workset-1".into(),
            team_id: "team-1".into(),
            index: 0,
            name: "Benchmark Workset".into(),
            description: None,
            comic_count: 1,
            created_at: archived_at,
            updated_at: archived_at,
        },
        chapter_snapshots,
    })
}

// Returns cached benchmark LabelPlus text for parse benchmarks.
fn label_plus_content() -> &'static str {
    // Cached benchmark LabelPlus text.
    static CONTENT: OnceLock<String> = OnceLock::new();

    CONTENT
        .get_or_init(|| {
            include_str!("../tests/materials/translations.lp.txt").repeat(64)
        })
        .as_str()
}

// Loads benchmark Poprako payload text used by import benchmarks.
fn poprako_content() -> &'static str {
    // Cached benchmark Poprako payload text.
    static CONTENT: OnceLock<String> = OnceLock::new();

    CONTENT
        .get_or_init(|| {
            //
            let unit_strings = (1..=2_000)
                .map(|index| {
                    //
                    format!(
                        "{{\"id\":\"unit-{}\",\"x\":1.0,\"y\":2.0,\"index_in_page\":{},\"is_inbox\":true,\"translated_text\":\"translated\",\"prooved_text\":\"proofread\",\"is_prooved\":true}}",
                        index,
                        index,
                    )
                })
                .collect::<Vec<_>>();

            let units = unit_strings.join(",");

            format!(
                "{{\"author\":\"benchmark\",\"title\":\"benchmark\",\"pages\":[{{\"image_filename\":\"001.png\",\"units\":[{}]}}]}}",
                units,
            )
        })
        .as_str()
}

// Builds a deterministic large payload reused by export benchmarks.
fn export_input() -> LabelPlusExportInput {
    //
    let archived_at = OffsetDateTime::UNIX_EPOCH;

    let mut pages = Vec::with_capacity(PAGE_COUNT * 8);

    let mut units_by_page_id = HashMap::new();

    for page_index in 0..(PAGE_COUNT * 8) {
        //
        let page_id = format!("page-{}", page_index);

        let unit_infos = (0..UNIT_COUNT)
            .map(|unit_index| {
                unit_info(&page_id, 0, page_index, unit_index, archived_at)
            })
            .collect();

        pages.push(PageInfo {
            id: page_id.clone(),
            chapter_id: "chapter-1".into(),
            index: page_index as i32,
            image_key: Some(format!("pages/{}.png", page_index)),
            is_image_uploaded: Some(true),
            image_version: Some(1),
            image_hash: Some(ImageHash::new([0u8; 32])),
            image_ext: Some(ImageExt::Png),
            total_unit_count: UNIT_COUNT as i32,
            translated_unit_count: UNIT_COUNT as i32,
            proofread_unit_count: UNIT_COUNT as i32,
            created_at: archived_at,
            updated_at: archived_at,
        });

        units_by_page_id.insert(page_id, unit_infos);
    }

    LabelPlusExportInput {
        pages,
        units_by_page_id,
    }
}

// Builds a deterministic user profile used in generated archive fixtures.
fn user_info(archived_at: OffsetDateTime) -> UserInfo {
    //
    UserInfo {
        id: "user-1".into(),
        qid: "benchmark-qid".into(),
        nickname: "benchmark-user".into(),
        avatar_key: None,
        is_avatar_uploaded: None,
        avatar_version: None,
        avatar_hash: None,
        avatar_ext: None,
        is_sadmin: false,
        last_active_at: archived_at,
        created_at: archived_at,
        updated_at: archived_at,
    }
}

// Builds one deterministic page unit used by both archive and export fixtures.
fn unit_info(
    page_id: &str,
    chapter_index: usize,
    page_index: usize,
    unit_index: usize,
    archived_at: OffsetDateTime,
) -> UnitInfo {
    //
    UnitInfo {
        id: format!("unit-{}-{}-{}", chapter_index, page_index, unit_index),
        page_id: page_id.into(),
        next_id: (unit_index + 1 < UNIT_COUNT).then(|| {
            format!("unit-{}-{}-{}", chapter_index, page_index, unit_index + 1,)
        }),
        is_bubble: unit_index.is_multiple_of(2),
        is_proofread: true,
        coord: UnitCoord {
            x_coord: unit_index as f64,
            y_coord: page_index as f64,
        },
        translated_text: Some(format!(
            "Translated text for chapter {}, page {}, unit {}.",
            chapter_index, page_index, unit_index,
        )),
        last_translator_id: Some("user-1".into()),
        proofread_text: Some(format!(
            "Proofread text for chapter {}, page {}, unit {}.",
            chapter_index, page_index, unit_index,
        )),
        last_proofreader_id: Some("user-1".into()),
        hidden_at: None,
        created_at: archived_at,
        updated_at: archived_at,
    }
}
