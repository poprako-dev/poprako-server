//! PostgreSQL correctness coverage for hierarchy mark-and-sweep deletion.

use diesel::{
    ExpressionMethods as _, Insertable, QueryDsl as _,
    TextExpressionMethods as _,
};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperStep as _};
use time::OffsetDateTime;

use poprako_rdb_core::RdbCore;

use crate::model::read::proj::subtree_delete::SubtreeDeleteSweepTarget;
use crate::model::write::chapter::ChapterEntry;
use crate::model::write::comic::ComicEntry;
use crate::model::write::page::PageEntry;
use crate::model::write::workset::WorksetEntry;
use crate::part::nucl::Serial;
use crate::part::repo::oper::subtree_delete::{
    ClaimSubtreeSweep, LockSubtreeDeleteScope, MarkSubtree, SubtreeRoot,
    SweepSubtree,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::chapter::ChapterEntryRow;
use crate::part_impl::repo::rdb_impl::entity::comic::ComicEntryRow;
use crate::part_impl::repo::rdb_impl::entity::page::PageEntryRow;
use crate::part_impl::repo::rdb_impl::entity::workset::WorksetEntryRow;
use crate::part_impl::repo::rdb_impl::schema::{
    t_chapter, t_comic, t_page, t_unit, t_workset,
};
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::value::subtree_delete::SubtreeSweepLevel;

const PREFIX: &str = "rdb-test-subtree-";

#[derive(Insertable)]
#[diesel(table_name = t_unit)]
struct UnitRow {
    f_id: String,

    f_page_id: String,
    f_next_id: Option<String>,
    f_hidden_at: Option<OffsetDateTime>,

    f_is_bubble: bool,
    f_is_proofread: bool,

    f_x_coord: f64,
    f_y_coord: f64,

    f_translated_text: Option<String>,
    f_last_translator_id: Option<String>,

    f_proofread_text: Option<String>,
    f_last_proofreader_id: Option<String>,

    f_created_at: OffsetDateTime,
    f_updated_at: OffsetDateTime,
}

#[derive(Clone, Copy)]
struct Scale {
    comics: usize,
    chapters_per_comic: usize,
    pages_per_chapter: usize,
    units_per_page: usize,
}

async fn seed_subtree(shared: &RdbCore, prefix: &str, scale: Scale) -> String {
    let fixture = test_shared::seed_workset(shared, prefix).await;
    let creator_id = format!("{prefix}user-owner");

    let comics = (0..scale.comics)
        .map(|comic_index| ComicEntry {
            id: format!("{prefix}comic-{comic_index:04}"),
            workset_id: fixture.workset_entry.id.clone(),
            index: comic_index,
            title: "Benchmark comic".into(),
            author: "Benchmark author".into(),
            description: None,
            creator_id: creator_id.clone(),
        })
        .collect::<Vec<_>>();

    let chapters = comics
        .iter()
        .flat_map(|comic| {
            let creator_id = creator_id.clone();

            (0..scale.chapters_per_comic).map(move |chapter_index| {
                ChapterEntry {
                    id: format!(
                        "{}chapter-{}-{chapter_index:04}",
                        prefix, comic.id
                    ),
                    comic_id: comic.id.clone(),
                    is_pinned: chapter_index == 0,
                    index: chapter_index,
                    subtitle: "Benchmark chapter".into(),
                    creator_id: creator_id.clone(),
                }
            })
        })
        .collect::<Vec<_>>();

    let pages = chapters
        .iter()
        .flat_map(|chapter| {
            (0..scale.pages_per_chapter).map(move |page_index| PageEntry {
                id: format!("{}page-{}-{page_index:04}", prefix, chapter.id),
                chapter_id: chapter.id.clone(),
                index: page_index,
            })
        })
        .collect::<Vec<_>>();

    let now = OffsetDateTime::now_utc();
    let units = pages
        .iter()
        .flat_map(|page| {
            (0..scale.units_per_page).map(move |unit_index| UnitRow {
                f_id: format!("{}unit-{}-{unit_index:04}", prefix, page.id),
                f_page_id: page.id.clone(),
                f_next_id: None,
                f_hidden_at: None,
                f_is_bubble: true,
                f_is_proofread: false,
                f_x_coord: 0.0,
                f_y_coord: 0.0,
                f_translated_text: None,
                f_last_translator_id: None,
                f_proofread_text: None,
                f_last_proofreader_id: None,
                f_created_at: now,
                f_updated_at: now,
            })
        })
        .collect::<Vec<_>>();

    let comic_rows = comics
        .iter()
        .map(ComicEntryRow::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let chapter_rows = chapters
        .iter()
        .map(ChapterEntryRow::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let page_rows = pages
        .iter()
        .map(PageEntryRow::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();

    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_comic::table)
        .values(&comic_rows)
        .execute(&mut conn)
        .await
        .unwrap();

    for rows in chapter_rows.chunks(1_000) {
        diesel::insert_into(t_chapter::table)
            .values(rows)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    for rows in page_rows.chunks(2_000) {
        diesel::insert_into(t_page::table)
            .values(rows)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    for rows in units.chunks(2_000) {
        diesel::insert_into(t_unit::table)
            .values(rows)
            .execute(&mut conn)
            .await
            .unwrap();
    }

    fixture.workset_entry.id
}

async fn mark_and_sweep_workset(shared: &RdbCore, workset_id: &str) {
    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    mark_workset(shared, workset_id).await;

    for level in [
        SubtreeSweepLevel::Chapter,
        SubtreeSweepLevel::Comic,
        SubtreeSweepLevel::Workset,
        SubtreeSweepLevel::Team,
    ] {
        loop {
            let swept = nucl
                .coord(async |context| {
                    let target = ClaimSubtreeSweep { level }
                        .step_on(&repo, context)
                        .await?;

                    let Some(target) = target else {
                        return Ok::<bool, BaseError>(false);
                    };

                    SweepSubtree { target: &target }
                        .step_on(&repo, context)
                        .await?;

                    Ok::<bool, BaseError>(true)
                })
                .await
                .unwrap();

            match swept {
                true => continue,
                false => break,
            }
        }
    }
}

async fn mark_workset(shared: &RdbCore, workset_id: &str) {
    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        let root = SubtreeRoot::Workset { id: workset_id };
        let scope = LockSubtreeDeleteScope { root }
            .step_on(&repo, context)
            .await?;

        MarkSubtree { scope: &scope }
            .step_on(&repo, context)
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();
}

pub async fn workset_subtree_delete_uses_mark_and_sweep(shared: RdbCore) {
    test_shared::reset(&shared, PREFIX).await;

    let workset_id = seed_subtree(
        &shared,
        PREFIX,
        Scale {
            comics: 2,
            chapters_per_comic: 2,
            pages_per_chapter: 3,
            units_per_page: 2,
        },
    )
    .await;

    mark_and_sweep_workset(&shared, &workset_id).await;

    let mut conn = shared.get().await.unwrap();
    let remaining_pages = t_page::table
        .filter(t_page::f_id.like(format!("{PREFIX}%")))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    let remaining_units = t_unit::table
        .filter(t_unit::f_id.like(format!("{PREFIX}%")))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(remaining_pages, 0);
    assert_eq!(remaining_units, 0);

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    explicit_comic_claim_does_not_short_circuit_on_chapter(&shared).await;

    comic_claim_is_stable_and_bounded(&shared).await;

    workset_claim_is_bounded(&shared).await;

    comic_batch_failure_rolls_back(&shared).await;
}

async fn explicit_comic_claim_does_not_short_circuit_on_chapter(
    shared: &RdbCore,
) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 2,
            chapters_per_comic: 1,
            pages_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        let target = ClaimSubtreeSweep {
            level: SubtreeSweepLevel::Chapter,
        }
        .step_on(&repo, context)
        .await?
        .unwrap();

        SweepSubtree { target: &target }
            .step_on(&repo, context)
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let target = nucl
        .coord(async |context| {
            ClaimSubtreeSweep {
                level: SubtreeSweepLevel::Comic,
            }
            .step_on(&repo, context)
            .await
        })
        .await
        .unwrap();

    assert_eq!(
        target,
        Some(SubtreeDeleteSweepTarget::Comics {
            ids: vec![format!("{PREFIX}comic-0000")],
        })
    );

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

async fn comic_claim_is_stable_and_bounded(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 65,
            chapters_per_comic: 0,
            pages_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());
    let target = nucl
        .coord(async |context| {
            ClaimSubtreeSweep {
                level: SubtreeSweepLevel::Comic,
            }
            .step_on(&repo, context)
            .await
        })
        .await
        .unwrap()
        .unwrap();

    let SubtreeDeleteSweepTarget::Comics { ids } = target else {
        panic!("comic level returned a non-comic claim");
    };

    let expected_ids = (0..64)
        .map(|index| format!("{PREFIX}comic-{index:04}"))
        .collect::<Vec<_>>();

    assert_eq!(ids, expected_ids);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

async fn workset_claim_is_bounded(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let fixture = test_shared::seed_workset(shared, PREFIX).await;
    let entries = (1..65)
        .map(|index| WorksetEntry {
            id: format!("{PREFIX}workset-{index:04}"),
            team_id: fixture.team_entry.id.clone(),
            index,
            name: format!("Workset {index}"),
            description: None,
        })
        .collect::<Vec<_>>();

    let rows = entries
        .iter()
        .map(WorksetEntryRow::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let deleted_at = OffsetDateTime::now_utc();
    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_workset::table)
        .values(&rows)
        .execute(&mut conn)
        .await
        .unwrap();

    diesel::update(
        t_workset::table.filter(t_workset::f_id.like(format!("{PREFIX}%"))),
    )
    .set(t_workset::f_deleted_at.eq(Some(deleted_at)))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());
    let target = nucl
        .coord(async |context| {
            ClaimSubtreeSweep {
                level: SubtreeSweepLevel::Workset,
            }
            .step_on(&repo, context)
            .await
        })
        .await
        .unwrap()
        .unwrap();

    let SubtreeDeleteSweepTarget::Worksets { ids } = target else {
        panic!("workset level returned a non-workset claim");
    };

    assert_eq!(ids.len(), 64);
    assert!(ids.windows(2).all(|pair| pair[0] < pair[1]));

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

async fn comic_batch_failure_rolls_back(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 2,
            chapters_per_comic: 1,
            pages_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let repo = HybRepo::new(shared.clone());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        let target = ClaimSubtreeSweep {
            level: SubtreeSweepLevel::Chapter,
        }
        .step_on(&repo, context)
        .await?
        .unwrap();

        SweepSubtree { target: &target }
            .step_on(&repo, context)
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let target = SubtreeDeleteSweepTarget::Comics {
        ids: vec![format!("{PREFIX}comic-0000"), format!("{PREFIX}comic-0001")],
    };
    let result = nucl
        .coord(async |context| {
            SweepSubtree { target: &target }
                .step_on(&repo, context)
                .await
        })
        .await;

    assert!(result.is_err());

    let mut conn = shared.get().await.unwrap();
    let comic_count = t_comic::table
        .filter(t_comic::f_id.like(format!("{PREFIX}%")))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(comic_count, 2);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}
