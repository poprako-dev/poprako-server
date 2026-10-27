//! PostgreSQL hierarchy sweep scenarios.

use super::*;

use crate::config::repo::RepoConfig;
use crate::part_impl::repo::rdb_impl::schema::t_page_source_raw_ident;
use diesel::expression::AsExpression;
use diesel::sql_types::Text;
use tokio::sync::oneshot;

// Use a non-default batch size to exercise configured claims and locking.
fn repo_config() -> RepoConfig {
    RepoConfig {
        subtree_sweep_batch_size: std::num::NonZeroU32::new(3).unwrap(),
        ..RepoConfig::default()
    }
}

pub async fn run(shared: RdbCore) {
    removes_a_complete_workset_subtree(&shared).await;
    explicit_comic_claim_does_not_short_circuit_on_chapter(&shared).await;
    comic_claim_is_stable_and_bounded(&shared).await;
    workset_claim_is_bounded(&shared).await;
    chapter_and_team_claim_one_row(&shared).await;
    comic_batch_failure_rolls_back(&shared).await;
}

async fn claim(
    shared: &RdbCore,
    level: SubtreeSweepLevel,
) -> Option<SubtreeDeleteSweepTarget> {
    let repo = RdbRepo::new(shared.clone(), repo_config());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        ClaimSubtreeSweep { level }.step_on(&repo, context).await
    })
    .await
    .unwrap()
}

async fn claim_and_sweep(
    shared: &RdbCore,
    level: SubtreeSweepLevel,
) -> Option<SubtreeDeleteSweepTarget> {
    let repo = RdbRepo::new(shared.clone(), repo_config());
    let nucl = RdbNucl::<Serial>::new(shared.clone());

    nucl.coord(async |context| {
        let target =
            ClaimSubtreeSweep { level }.step_on(&repo, context).await?;

        let Some(target) = target else {
            return Ok::<_, BaseError>(None);
        };

        SweepSubtree { target: &target }
            .step_on(&repo, context)
            .await?;

        Ok::<_, BaseError>(Some(target))
    })
    .await
    .unwrap()
}

async fn removes_a_complete_workset_subtree(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 2,
            chapters_per_comic: 2,
            page_sources_per_chapter: 3,
            units_per_page: 2,
        },
    )
    .await;

    {
        let mut conn = shared.get().await.unwrap();

        diesel::insert_into(t_page_source_raw_ident::table)
            .values(
                t_page_source::table
                    .filter(t_page_source::f_id.like([PREFIX, "%"].concat()))
                    .select((
                        t_page_source::f_id,
                        <&str as AsExpression<Text>>::as_expression(
                            "source.png",
                        ),
                    )),
            )
            .into_columns((
                t_page_source_raw_ident::f_page_source_id,
                t_page_source_raw_ident::f_raw_ident,
            ))
            .execute(&mut conn)
            .await
            .unwrap();
    }

    mark_and_sweep_workset(shared, &workset_id).await;

    let mut conn = shared.get().await.unwrap();
    let remaining_page_sources = t_page_source::table
        .filter(t_page_source::f_id.like([PREFIX, "%"].concat()))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    let remaining_units = t_unit::table
        .filter(t_unit::f_id.like([PREFIX, "%"].concat()))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(remaining_page_sources, 0);
    assert_eq!(remaining_units, 0);

    let remaining_raw_idents = t_page_source_raw_ident::table
        .filter(
            t_page_source_raw_ident::f_page_source_id
                .like([PREFIX, "%"].concat()),
        )
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(remaining_raw_idents, 0);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
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
            page_sources_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let repo = RdbRepo::new(shared.clone(), repo_config());
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
            ids: vec![[PREFIX, "comic-0000"].concat()],
        })
    );

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
async fn comic_claim_is_stable_and_bounded(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 4,
            chapters_per_comic: 0,
            page_sources_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let (claimed_sink, claimed_stream) = oneshot::channel();
    let (release_sink, release_stream) = oneshot::channel();
    let claim_core = shared.clone();
    let claim_task = tokio::spawn(async move {
        let repo = RdbRepo::new(claim_core.clone(), repo_config());
        let nucl = RdbNucl::<Serial>::new(claim_core);

        nucl.coord(async move |context| {
            let target = ClaimSubtreeSweep {
                level: SubtreeSweepLevel::Comic,
            }
            .step_on(&repo, context)
            .await?
            .unwrap();

            claimed_sink.send(target).unwrap();

            release_stream.await.unwrap();

            Ok::<(), BaseError>(())
        })
        .await
        .unwrap();
    });
    let locked_target = claimed_stream.await.unwrap();
    let concurrent_target = claim(shared, SubtreeSweepLevel::Comic).await;
    let SubtreeDeleteSweepTarget::Comics { ids: locked_ids } = locked_target
    else {
        panic!("comic level returned a non-comic claim");
    };

    assert_eq!(locked_ids.len(), 3);
    assert_eq!(
        concurrent_target,
        Some(SubtreeDeleteSweepTarget::Comics {
            ids: vec![[PREFIX, "comic-0003"].concat()],
        })
    );

    release_sink.send(()).unwrap();

    claim_task.await.unwrap();

    let target = claim_and_sweep(shared, SubtreeSweepLevel::Comic)
        .await
        .unwrap();
    let SubtreeDeleteSweepTarget::Comics { ids } = target else {
        panic!("comic level returned a non-comic claim");
    };

    let expected_ids = (0..3)
        .map(|index| [PREFIX, "comic-", &format!("{:04}", &index)].concat())
        .collect::<Vec<_>>();

    assert_eq!(ids, expected_ids);

    assert_eq!(
        claim_and_sweep(shared, SubtreeSweepLevel::Comic).await,
        Some(SubtreeDeleteSweepTarget::Comics {
            ids: vec![[PREFIX, "comic-0003"].concat()],
        })
    );
    assert_eq!(claim(shared, SubtreeSweepLevel::Comic).await, None);

    let mut conn = shared.get().await.unwrap();
    let comic_count = t_comic::table
        .filter(t_comic::f_id.like([PREFIX, "%"].concat()))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(comic_count, 0);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

#[expect(
    clippy::panic,
    reason = "This test fails explicitly when an expected fixture variant or assertion is violated"
)]
async fn workset_claim_is_bounded(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let fixture = test_shared::seed_workset(shared, PREFIX).await;

    let entries = (1..=4)
        .map(|index| WorksetEntry {
            id: [PREFIX, "workset-", &format!("{:04}", &index)].concat(),
            team_id: fixture.team_entry.id.clone(),
            index,
            name: ["Workset ", &index.to_string()].concat(),
            description: None,
        })
        .collect::<Vec<_>>();
    let items = entries
        .iter()
        .map(WorksetItem::try_from)
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    let deleted_at = OffsetDateTime::now_utc();
    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_workset::table)
        .values(&items)
        .execute(&mut conn)
        .await
        .unwrap();

    let comic_entry = ComicEntryShape {
        id: [PREFIX, "blocked-comic"].concat(),
        workset_id: &fixture.workset_entry.id,
        index: 0,
        title: "Blocked comic".into(),
        author: "Author".into(),
        description: None,
        creator_id: &[PREFIX, "user-owner"].concat(),
    };
    let comic_item = ComicItem::try_from(&comic_entry).unwrap();

    diesel::insert_into(t_comic::table)
        .values(&comic_item)
        .execute(&mut conn)
        .await
        .unwrap();

    diesel::update(
        t_workset::table.filter(t_workset::f_id.like([PREFIX, "%"].concat())),
    )
    .set(t_workset::f_deleted_at.eq(Some(deleted_at)))
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    let target = claim_and_sweep(shared, SubtreeSweepLevel::Workset)
        .await
        .unwrap();

    let SubtreeDeleteSweepTarget::Worksets { ids } = target else {
        panic!("workset level returned a non-workset claim");
    };

    assert_eq!(ids.len(), 3);
    assert!(
        ids.windows(2)
            .all(|pair| (*pair.first().unwrap()) < (*pair.get(1).unwrap()))
    );

    assert_eq!(
        claim_and_sweep(shared, SubtreeSweepLevel::Workset).await,
        Some(SubtreeDeleteSweepTarget::Worksets {
            ids: vec![[PREFIX, "workset-0004"].concat()],
        })
    );
    assert_eq!(claim(shared, SubtreeSweepLevel::Workset).await, None);

    let mut conn = shared.get().await.unwrap();
    let workset_ids = t_workset::table
        .filter(t_workset::f_id.like([PREFIX, "%"].concat()))
        .select(t_workset::f_id)
        .load::<String>(&mut conn)
        .await
        .unwrap();

    assert_eq!(workset_ids, vec![fixture.workset_entry.id]);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}

async fn chapter_and_team_claim_one_row(shared: &RdbCore) {
    test_shared::reset(shared, PREFIX).await;

    let workset_id = seed_subtree(
        shared,
        PREFIX,
        Scale {
            comics: 1,
            chapters_per_comic: 2,
            page_sources_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let first_chapter =
        claim_and_sweep(shared, SubtreeSweepLevel::Chapter).await;
    let second_chapter =
        claim_and_sweep(shared, SubtreeSweepLevel::Chapter).await;

    assert!(matches!(
        first_chapter,
        Some(SubtreeDeleteSweepTarget::Chapter { .. })
    ));
    assert!(matches!(
        second_chapter,
        Some(SubtreeDeleteSweepTarget::Chapter { .. })
    ));
    assert_eq!(claim(shared, SubtreeSweepLevel::Chapter).await, None);

    test_shared::cleanup(shared, PREFIX).await.unwrap();

    let first_fixture =
        test_shared::seed_user_and_team(shared, &[PREFIX, "a-"].concat()).await;
    let second_fixture =
        test_shared::seed_user_and_team(shared, &[PREFIX, "b-"].concat()).await;
    let team_ids =
        vec![first_fixture.team_entry.id, second_fixture.team_entry.id];
    let mut conn = shared.get().await.unwrap();

    diesel::update(t_team::table.filter(t_team::f_id.eq_any(&team_ids)))
        .set(t_team::f_deleted_at.eq(Some(OffsetDateTime::now_utc())))
        .execute(&mut conn)
        .await
        .unwrap();

    drop(conn);

    for team_id in team_ids {
        assert_eq!(
            claim_and_sweep(shared, SubtreeSweepLevel::Team).await,
            Some(SubtreeDeleteSweepTarget::Team { id: team_id })
        );
    }

    assert_eq!(claim(shared, SubtreeSweepLevel::Team).await, None);

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
            page_sources_per_chapter: 0,
            units_per_page: 0,
        },
    )
    .await;

    mark_workset(shared, &workset_id).await;

    let repo = RdbRepo::new(shared.clone(), repo_config());
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

    let archive_record = ComicArchiveRecord {
        id: [PREFIX, "archive"].concat(),
        team_id: [PREFIX, "team"].concat(),
        source_comic_id: [PREFIX, "comic-0000"].concat(),
        archived_payload: "{}".into(),
        archiver_id: [PREFIX, "user-owner"].concat(),
        created_at: OffsetDateTime::now_utc(),
    };
    let archive_item = ComicArchiveItem::from(&archive_record);
    let mut conn = shared.get().await.unwrap();

    diesel::insert_into(t_comic_archive::table)
        .values(&archive_item)
        .execute(&mut conn)
        .await
        .unwrap();

    drop(conn);

    let target = SubtreeDeleteSweepTarget::Comics {
        ids: vec![
            [PREFIX, "comic-0000"].concat(),
            [PREFIX, "comic-0001"].concat(),
        ],
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
        .filter(t_comic::f_id.like([PREFIX, "%"].concat()))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();
    let archive_count = t_comic_archive::table
        .filter(t_comic_archive::f_id.eq(&archive_record.id))
        .count()
        .get_result::<i64>(&mut conn)
        .await
        .unwrap();

    assert_eq!(comic_count, 2);
    assert_eq!(archive_count, 1);

    diesel::delete(
        t_comic_archive::table
            .filter(t_comic_archive::f_id.eq(&archive_record.id)),
    )
    .execute(&mut conn)
    .await
    .unwrap();

    drop(conn);

    test_shared::cleanup(shared, PREFIX).await.unwrap();
}
