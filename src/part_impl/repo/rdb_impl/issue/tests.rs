#![allow(
    clippy::unwrap_used,
    clippy::expect_used,
    reason = "Persistence fixtures and assertions fail immediately on violated invariants"
)]

//! Typed persistence, replacement rollback, constraints and lifecycle tests.

use diesel::prelude::{ExpressionMethods as _, QueryDsl as _};
use diesel_async::RunQueryDsl as _;
use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};
use time::OffsetDateTime;

use poprako_rdb_core::RdbCore;

use crate::complex::comic_archive as comic_archive_complex;
use crate::model::shared::issue::IssueRect;
use crate::model::write::issue::{ChapterIssuesRepl, IssueEntry};
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter::{
    GetChapterInfoExcluded, SetChapterConfirmedArtworkVersion,
};
use crate::part::repo::oper::comic::TouchComicLastActive;
use crate::part::repo::oper::comic_archive::{
    CommitComicArchive, GetComicArchiveSnapshotExcluded,
};
use crate::part::repo::oper::issue::{
    ClearChapterIssues, ListIssueInfos, ReplaceChapterIssues,
};
use crate::part::repo::oper::page::{DeletePages, ListPageInfosExcluded};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::entity::issue::IssueRow;
use crate::part_impl::repo::rdb_impl::schema::t_issue;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::test_rdb::start;

fn entry(id: &str, page_id: &str, index: i32) -> IssueEntry {
    IssueEntry {
        id: id.into(),
        page_id: page_id.into(),
        index,
        variant: "custom category".into(),
        layer_path: Some("opaque path".into()),
        rect: Some(IssueRect {
            x_coord: 0.1,
            y_coord: 0.2,
            width: 0.3,
            height: 0.1,
        }),
        note: "first line\nsecond line".into(),
    }
}

async fn replace(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    chapter_id: &str,
    entries: &[IssueEntry],
) -> BaseRest<()> {
    nucl.coord(async |context| {
        let chapter_info = GetChapterInfoExcluded {
            id: chapter_id,
            incls: &[],
        }
        .step_on(repo, context)
        .await?;

        ListPageInfosExcluded { chapter_id }
            .step_on(repo, context)
            .await?;

        let chapter_issues_repl = ChapterIssuesRepl {
            chapter_id,
            entries,
        };

        ReplaceChapterIssues {
            repl: &chapter_issues_repl,
        }
        .step_on(repo, context)
        .await?;

        TouchComicLastActive {
            id: &chapter_info.comic_id,
        }
        .step_on(repo, context)
        .await?;

        accept(())
    })
    .await
    .map_err(BaseError::from)
}

// review_constraints(t_issue)(negative): the database rejects partial, nonfinite, out-of-page and duplicate-index geometry.
async fn verify_constraints(shared: &RdbCore, page_id: &str) {
    let mut conn = shared.get().await.unwrap();

    for variant in [
        "partial",
        "negative-index",
        "infinite",
        "nan",
        "outside",
        "zero-size",
    ] {
        let mut row = IssueRow::from(&entry("review-invalid", page_id, 3));

        match variant {
            "partial" => row.f_height = None,
            "negative-index" => row.f_index = -1,
            "infinite" => row.f_width = Some(f64::INFINITY),
            "nan" => row.f_x_coord = Some(f64::NAN),
            "outside" => row.f_width = Some(1.0),
            _ => row.f_width = Some(0.0),
        }

        assert!(
            diesel::insert_into(t_issue::table)
                .values(&row)
                .execute(&mut conn)
                .await
                .is_err()
        );
    }

    assert_eq!(
        t_issue::table
            .filter(t_issue::f_page_id.eq(page_id))
            .count()
            .get_result::<i64>(&mut conn)
            .await
            .unwrap(),
        2
    );
}

// review_archive(GetComicArchiveSnapshotExcluded)(positive): ordered current issues are captured in the immutable Page snapshot.
async fn verify_archive(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    chapter_id: &str,
    page_id: &str,
) {
    let snapshot = nucl
        .coord(async |context| {
            let chapter = GetChapterInfoExcluded {
                id: chapter_id,
                incls: &[],
            }
            .step_on(repo, context)
            .await?;

            GetComicArchiveSnapshotExcluded {
                comic_id: &chapter.comic_id,
            }
            .step_on(repo, context)
            .await
        })
        .await
        .unwrap();

    let page = snapshot
        .chapter_snapshots
        .as_slice()
        .first()
        .unwrap()
        .page_snapshots
        .as_slice()
        .first()
        .unwrap();

    assert_eq!(page.page_info.id, page_id);

    assert_eq!(page.issue_infos.len(), 2100);

    assert_eq!(page.issue_infos.as_slice().first().unwrap().index, 0);

    let archiver_id = snapshot
        .chapter_snapshots
        .as_slice()
        .first()
        .unwrap()
        .chapter_info
        .creator_id
        .clone();

    let entry = comic_archive_complex::prepare_entry(
        snapshot,
        archiver_id,
        OffsetDateTime::now_utc(),
    )
    .await
    .unwrap();

    let payload: serde_json::Value =
        serde_json::from_str(&entry.record.archived_payload).unwrap();

    let issues = payload
        .get("chapters")
        .unwrap()
        .as_array()
        .unwrap()
        .as_slice()
        .first()
        .unwrap()
        .get("pages")
        .unwrap()
        .as_array()
        .unwrap()
        .as_slice()
        .first()
        .unwrap()
        .get("issues")
        .unwrap()
        .as_array()
        .unwrap();

    assert_eq!(issues.len(), 2100);

    let rollback = nucl
        .coord(async |context| {
            CommitComicArchive { entry: &entry }
                .step_on(repo, context)
                .await?;

            let active_count = t_issue::table
                .filter(t_issue::f_page_id.eq(page_id))
                .count()
                .get_result::<i64>(context.conn())
                .await
                .map_err(crate::shared::result::diesel)?;

            assert_eq!(active_count, 0);

            Err::<(), _>(BaseError::Unrecoverable {
                msg: "rollback archive fixture".into(),
            })
        })
        .await;

    assert!(rollback.is_err());

    assert_eq!(
        ListIssueInfos { page_id }.run_on(repo).await.unwrap().len(),
        2100
    );
}

// review_storage(ReplaceChapterIssues)(positive): complete replacement preserves all values and is bounded for large batches.
#[tokio::test]
#[serial_test::serial(repo_rdb)]
async fn issues_roundtrip_rollback_constraints_and_cleanup() {
    let test_rdb = start().await;

    let shared = test_rdb.core();

    let fixture = test_shared::seed_page(&shared, "rdb-test-review-").await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let chapter_id = &fixture.chapter_entry.id;

    let page_id = &fixture.page_entry.id;

    let mut entries = vec![
        entry("review-first", page_id, 0),
        entry("review-second", page_id, 1),
    ];

    entries.first_mut().unwrap().layer_path = None;

    entries.first_mut().unwrap().rect = None;

    replace(&repo, &nucl, chapter_id, &entries).await.unwrap();

    let baseline = ListIssueInfos { page_id }.run_on(&repo).await.unwrap();

    assert_eq!(baseline.len(), 2);

    assert_eq!(baseline.as_slice().first().unwrap().index, 0);

    assert!(baseline.as_slice().first().unwrap().rect.is_none());

    assert_eq!(baseline.get(1).unwrap().rect, entries.get(1).unwrap().rect);

    assert_eq!(baseline.get(1).unwrap().note, "first line\nsecond line");

    let invalid_entries = vec![
        entry("review-duplicate-1", page_id, 0),
        entry("review-duplicate-2", page_id, 0),
    ];

    assert!(
        replace(&repo, &nucl, chapter_id, &invalid_entries)
            .await
            .is_err()
    );

    assert_eq!(
        ListIssueInfos { page_id }.run_on(&repo).await.unwrap(),
        baseline
    );

    let invalid_entries = vec![entry("review-invalid-fk", "missing-page", 0)];

    assert!(
        replace(&repo, &nucl, chapter_id, &invalid_entries)
            .await
            .is_err()
    );

    assert_eq!(
        ListIssueInfos { page_id }.run_on(&repo).await.unwrap(),
        baseline
    );

    verify_constraints(&shared, page_id).await;

    let failure = nucl
        .coord(async |context| {
            ClearChapterIssues { chapter_id }
                .step_on(&repo, context)
                .await?;

            Err::<(), _>(BaseError::Unrecoverable {
                msg: "injected failure after clearing review".into(),
            })
        })
        .await;

    assert!(failure.is_err());

    assert_eq!(
        ListIssueInfos { page_id }.run_on(&repo).await.unwrap(),
        baseline
    );

    nucl.coord(async |context| {
        SetChapterConfirmedArtworkVersion {
            id: chapter_id,
            version: 42,
        }
        .step_on(&repo, context)
        .await?;

        let chapter = GetChapterInfoExcluded {
            id: chapter_id,
            incls: &[],
        }
        .step_on(&repo, context)
        .await?;

        assert_eq!(chapter.confirmed_artwork_ver, Some(42));

        accept(())
    })
    .await
    .unwrap();

    let first = (0..2100)
        .map(|index| entry(&format!("review-a-{index}"), page_id, index))
        .collect::<Vec<_>>();

    let second = (0..2100)
        .map(|index| entry(&format!("review-b-{index}"), page_id, index))
        .collect::<Vec<_>>();

    let (first_result, second_result) = tokio::join!(
        replace(&repo, &nucl, chapter_id, &first),
        replace(&repo, &nucl, chapter_id, &second)
    );

    assert!(first_result.is_ok() || second_result.is_ok());

    let issues = ListIssueInfos { page_id }.run_on(&repo).await.unwrap();

    assert_eq!(issues.len(), 2100);

    let batch = issues
        .as_slice()
        .first()
        .unwrap()
        .id
        .split('-')
        .nth(1)
        .unwrap();

    assert!(
        issues
            .iter()
            .all(|issue| issue.id.split('-').nth(1) == Some(batch))
    );

    verify_archive(&repo, &nucl, chapter_id, page_id).await;

    replace(&repo, &nucl, chapter_id, &[]).await.unwrap();

    assert!(
        ListIssueInfos { page_id }
            .run_on(&repo)
            .await
            .unwrap()
            .is_empty()
    );

    replace(
        &repo,
        &nucl,
        chapter_id,
        &[entry("review-final", page_id, 0)],
    )
    .await
    .unwrap();

    nucl.coord(async |context| {
        DeletePages::Chapter { chapter_id }
            .step_on(&repo, context)
            .await
    })
    .await
    .unwrap();

    assert!(
        ListIssueInfos { page_id }
            .run_on(&repo)
            .await
            .unwrap()
            .is_empty()
    );

    test_shared::cleanup(&shared, "rdb-test-review-")
        .await
        .unwrap();
}
