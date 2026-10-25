use super::*;

use crate::part::repo::oper::comic_archive::ListComicArchivePayloads;
use crate::result::BaseRest;
use crate::shared::RdbContext;
use crate::value::comic_archive::ComicArchiveMonth;

const EXPORT_PREFIX: &str = "rdb-test-archive-export-";

// Insert a small normal archive record through typed schema expressions.
async fn insert_payload(
    shared: &RdbCore,
    team_id: &str,
    id: &str,
    created_at: OffsetDateTime,
    payload: &str,
) {
    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_comic_archive::table)
        .values((
            t_comic_archive::f_id.eq(id),
            t_comic_archive::f_team_id.eq(team_id),
            t_comic_archive::f_source_comic_id.eq(id),
            t_comic_archive::f_archived_payload.eq(payload),
            t_comic_archive::f_archiver_id.eq("export-fixture"),
            t_comic_archive::f_created_at.eq(created_at),
        ))
        .execute(&mut conn)
        .await
        .unwrap();
}

/// Verifies typed month filtering, deterministic chunks, byte bounds, and snapshot consistency.
///
/// # Panics
/// Panics when fixture setup or an export invariant fails.
pub async fn bounded_export_payloads(shared: RdbCore) {
    test_shared::reset(&shared, EXPORT_PREFIX).await;

    let fixture = test_shared::seed_user_and_team(&shared, EXPORT_PREFIX).await;

    let months = ComicArchiveMonth::parse_retained(
        vec!["2025-01".into(), "2025-03".into()],
        OffsetDateTime::now_utc(),
    )
    .unwrap();

    let january = months.as_slice().first().unwrap().start;

    for index in (0..65).rev() {
        insert_payload(
            &shared,
            &fixture.team_entry.id,
            &format!("{EXPORT_PREFIX}{index:03}"),
            january,
            "中",
        )
        .await;
    }

    insert_payload(
        &shared,
        &fixture.team_entry.id,
        &format!("{EXPORT_PREFIX}gap"),
        months.as_slice().first().unwrap().end,
        "gap payload exceeds the tiny test batch",
    )
    .await;

    insert_payload(
        &shared,
        &fixture.team_entry.id,
        &format!("{EXPORT_PREFIX}march"),
        months.get(1).unwrap().start,
        "end",
    )
    .await;

    let (nucl, repo) = (
        RdbNucl::<ReptRead>::new(shared.clone()),
        RdbRepo::new(shared.clone(), RepoConfig::default()),
    );

    nucl.coord(async |context| {
        verify_snapshot_keyset_traversal(
            &repo,
            &shared,
            context,
            &fixture.team_entry.id,
            &months,
        )
        .await?;

        verify_payload_budget_and_team_scope(
            &repo,
            context,
            &fixture.team_entry.id,
            &months,
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let oversized = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ListComicArchivePayloads {
                    team_id: &fixture.team_entry.id,
                    months: &months,
                    after: None,
                    payload_budget_bytes: 2,
                },
            )
            .await
        })
        .await;

    assert!(matches!(
        oversized,
        Err(poprako_orchestra::nucl::Error::Step(
            BaseError::Unavailable { .. }
        ))
    ));

    {
        let mut conn = shared.get().await.unwrap();

        diesel::delete(
            t_comic_archive::table
                .filter(t_comic_archive::f_team_id.eq(&fixture.team_entry.id)),
        )
        .execute(&mut conn)
        .await
        .unwrap();
    }

    test_shared::cleanup(&shared, EXPORT_PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, EXPORT_PREFIX)
        .await
        .unwrap();
}

// Traverse tied archive timestamps while excluding a commit between keyset pages.
async fn verify_snapshot_keyset_traversal(
    repo: &RdbRepo,
    shared: &RdbCore,
    context: &mut RdbContext<ReptRead>,
    team_id: &str,
    months: &[ComicArchiveMonth],
) -> BaseRest<()> {
    let first = repo
        .step(
            context,
            &ListComicArchivePayloads {
                team_id,
                months,
                after: None,
                payload_budget_bytes: 1024,
            },
        )
        .await?;

    assert_eq!(first.len(), 64);

    assert_eq!(
        first.as_slice().first().unwrap().id,
        format!("{EXPORT_PREFIX}000")
    );

    let last = first.last().unwrap();

    // The independent commit remains between queries in this same snapshot.
    insert_payload(
        shared,
        team_id,
        &format!("{EXPORT_PREFIX}999"),
        months.first().unwrap().start,
        "new",
    )
    .await;

    let second = repo
        .step(
            context,
            &ListComicArchivePayloads {
                team_id,
                months,
                after: Some((last.created_at, &last.id)),
                payload_budget_bytes: 1024,
            },
        )
        .await?;

    assert_eq!(second.len(), 2);

    assert_eq!(
        second.as_slice().first().unwrap().id,
        format!("{EXPORT_PREFIX}064")
    );

    assert_eq!(second.get(1).unwrap().payload, "end");

    Ok(())
}

// Verify the combined UTF-8 byte budget and isolation from another team.
async fn verify_payload_budget_and_team_scope(
    repo: &RdbRepo,
    context: &mut RdbContext<ReptRead>,
    team_id: &str,
    months: &[ComicArchiveMonth],
) -> BaseRest<()> {
    let bounded = repo
        .step(
            context,
            &ListComicArchivePayloads {
                team_id,
                months,
                after: None,
                payload_budget_bytes: 6,
            },
        )
        .await?;

    assert_eq!(bounded.len(), 2);

    assert_eq!(
        bounded
            .iter()
            .map(|record| record.payload.len())
            .sum::<usize>(),
        6
    );

    let missing = repo
        .step(
            context,
            &ListComicArchivePayloads {
                team_id: "another-team",
                months,
                after: None,
                payload_budget_bytes: 1024,
            },
        )
        .await?;

    assert!(missing.is_empty());

    Ok(())
}
