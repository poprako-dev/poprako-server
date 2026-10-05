#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// comic_roundtrip_uses_testcontainer(ComicRepo)(positive): comic repo persists, lists by one-based display index, and refreshes composed search after update.

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::chapter::{ChapterEntry, ChapterStageRepl};
use crate::model::write::comic::{ComicEntry, ComicRepl};
use crate::model::write::workset::WorksetEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter::{CreateChapter, UpdateChapterStage};
use crate::part::repo::oper::comic::{
    CreateComic, GetComicInfo, ListComicInfos, UpdateComic,
};
use crate::part::repo::oper::workset::CreateWorkset;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::ComicFixture;
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::comic::ComicInclOpt;

const PREFIX: &str = "rdb-test-comic-domain-";

// Verify requested relation inclusions before mutating the comic.
async fn verify_related_workset_team(
    repo: &HybRepo,
    comic_fixture: &ComicFixture,
) {
    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("Comic".into()),
        stages: None,
        status: None,
        incl_opt: vec![ComicInclOpt::WorksetTeam],
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let comic_infos = repo
        .run(&ListComicInfos {
            spec: &comic_list_spec,
        })
        .await
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(
        comic_infos
            .as_slice()
            .first()
            .unwrap()
            .workset
            .as_ref()
            .unwrap()
            .id,
        comic_fixture.workset_entry.id
    );

    assert_eq!(
        comic_infos
            .as_slice()
            .first()
            .unwrap()
            .team
            .as_ref()
            .unwrap()
            .id,
        comic_fixture.team_entry.id
    );
}

// Verify updated title, author search, and one-based display index search.
async fn verify_updated_search(repo: &HybRepo, comic_fixture: &ComicFixture) {
    let comic_info_update = ComicRepl {
        id: comic_fixture.comic_entry.id.clone(),
        title: "RDB Comic Updated".into(),
        author: "RDB Author Updated".into(),
        description: Some("updated".into()),
    };

    repo.run(&UpdateComic {
        update: &comic_info_update,
    })
    .await
    .unwrap();

    let comic_info = repo
        .run(&GetComicInfo {
            id: &comic_fixture.comic_entry.id,
            incls: &[],
        })
        .await
        .unwrap();

    assert_eq!(comic_info.title, "RDB Comic Updated");

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("RDB Author Updated".into()),
        stages: None,
        status: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let comic_infos = repo
        .run(&ListComicInfos {
            spec: &comic_list_spec,
        })
        .await
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(
        comic_infos.as_slice().first().unwrap().id,
        comic_fixture.comic_entry.id
    );

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("1".into()),
        stages: None,
        status: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let comic_infos = repo
        .run(&ListComicInfos {
            spec: &comic_list_spec,
        })
        .await
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(comic_infos.as_slice().first().unwrap().index, 0);
}

// Verify SQL wildcard characters are treated as literal search text.
async fn verify_literal_search(repo: &HybRepo, comic_fixture: &ComicFixture) {
    for fuzzy_title in ["%", "_", "\\"] {
        //
        let comic_list_spec = ComicListSpec {
            workset_id: comic_fixture.workset_entry.id.clone(),
            fuzzy_title: Some(fuzzy_title.into()),
            stages: None,
            status: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        };

        let comic_infos = repo
            .run(&ListComicInfos {
                spec: &comic_list_spec,
            })
            .await
            .unwrap();

        assert!(comic_infos.is_empty());
    }

    let comic_info_update = ComicRepl {
        id: comic_fixture.comic_entry.id.clone(),
        title: "RDB 100%_Comic\\Updated".into(),
        author: "RDB Author Updated".into(),
        description: Some("updated".into()),
    };

    repo.run(&UpdateComic {
        update: &comic_info_update,
    })
    .await
    .unwrap();

    for fuzzy_title in ["%_", "\\Updated"] {
        //
        let comic_list_spec = ComicListSpec {
            workset_id: comic_fixture.workset_entry.id.clone(),
            fuzzy_title: Some(fuzzy_title.into()),
            stages: None,
            status: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        };

        let comic_infos = repo
            .run(&ListComicInfos {
                spec: &comic_list_spec,
            })
            .await
            .unwrap();

        assert_eq!(comic_infos.len(), 1);
    }
}

// Seed another workset to ensure stage filters do not leak across worksets.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn seed_sibling_stages(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
) {
    let sibling_workset = WorksetEntry {
        id: format!("{}sibling-workset", PREFIX),
        team_id: comic_fixture.team_entry.id.clone(),
        index: 1,
        name: "Sibling Workset".into(),
        description: None,
    };

    let sibling_comic = ComicEntry {
        id: format!("{}sibling-comic", PREFIX),
        workset_id: sibling_workset.id.clone().into(),
        index: 0,
        title: "Sibling Comic".into(),
        author: "Sibling Author".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone().into(),
    };

    let sibling_chapter = ChapterEntry {
        id: format!("{}sibling-chapter", PREFIX),
        comic_id: sibling_comic.id.clone().into(),
        is_pinned: true,
        index: 0,
        subtitle: "Sibling Chapter".into(),
        creator_id: comic_fixture.creator_form.id.clone().into(),
    };

    let sibling_stage_update = ChapterStageRepl {
        id: sibling_chapter.id.clone(),
        stages: StageMask::try_from(0)
            .unwrap()
            .try_set_phase(Stage::Translate, StagePhase::Active)
            .unwrap()
            .try_set_phase(Stage::Proofread, StagePhase::Completed)
            .unwrap(),
    };

    nucl.coord(async |context| {
        repo.step(
            &mut *context,
            &CreateWorkset {
                entry: &sibling_workset,
            },
        )
        .await?;

        repo.step(
            &mut *context,
            &CreateComic {
                entry: &sibling_comic,
            },
        )
        .await?;

        repo.step(
            &mut *context,
            &CreateChapter {
                entry: &sibling_chapter,
            },
        )
        .await?;

        repo.step(
            context,
            &UpdateChapterStage {
                update: &sibling_stage_update,
            },
        )
        .await?;

        Ok::<(), crate::result::BaseError>(())
    })
    .await
    .unwrap();
}

// Exercise every legal single-stage phase against the pinned chapter.
async fn verify_single_stage_filters(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    chapter_entry: &ChapterEntry<'_>,
) {
    let legal_stage_phases = [
        (Stage::RawProvide, StagePhase::Pending),
        (Stage::RawProvide, StagePhase::Completed),
        (Stage::Translate, StagePhase::Pending),
        (Stage::Translate, StagePhase::Active),
        (Stage::Translate, StagePhase::Completed),
        (Stage::Proofread, StagePhase::Pending),
        (Stage::Proofread, StagePhase::Active),
        (Stage::Proofread, StagePhase::Completed),
        (Stage::TypesetRedraw, StagePhase::Pending),
        (Stage::TypesetRedraw, StagePhase::Active),
        (Stage::TypesetRedraw, StagePhase::Completed),
        (Stage::Review, StagePhase::Pending),
        (Stage::Review, StagePhase::Completed),
        (Stage::Publish, StagePhase::Pending),
        (Stage::Publish, StagePhase::Completed),
    ];

    for (stage, phase) in legal_stage_phases {
        let stages = StageMask::try_from(0)
            .unwrap()
            .try_set_phase(stage, phase)
            .unwrap();

        let stage_update = ChapterStageRepl {
            id: chapter_entry.id.clone(),
            stages,
        };

        nucl.coord(async |context| {
            repo.step(
                context,
                &UpdateChapterStage {
                    update: &stage_update,
                },
            )
            .await
        })
        .await
        .unwrap();

        let comic_list_spec = ComicListSpec {
            workset_id: comic_fixture.workset_entry.id.clone(),
            fuzzy_title: None,
            stages: Some(single_stage_filter(stage, phase)),
            status: None,
            incl_opt: Vec::new(),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        };

        let comic_infos = repo
            .run(&ListComicInfos {
                spec: &comic_list_spec,
            })
            .await
            .unwrap();

        assert_eq!(comic_infos.len(), 1, "{stage:?} {phase:?}");
    }
}

// Verify intersected stage filters and the unrestricted stage mask.
async fn verify_combined_stage_filters(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    chapter_entry: &ChapterEntry<'_>,
) {
    let combined_stages = StageMask::try_from(0)
        .unwrap()
        .try_set_phase(Stage::Translate, StagePhase::Active)
        .unwrap()
        .try_set_phase(Stage::Proofread, StagePhase::Completed)
        .unwrap();

    let combined_update = ChapterStageRepl {
        id: chapter_entry.id.clone(),
        stages: combined_stages,
    };

    nucl.coord(async |context| {
        repo.step(
            context,
            &UpdateChapterStage {
                update: &combined_update,
            },
        )
        .await
    })
    .await
    .unwrap();

    let combined_filter_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: None,
        stages: Some(stage_filter(&[
            (Stage::Translate, StagePhase::Active),
            (Stage::Proofread, StagePhase::Completed),
        ])),
        status: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let combined_comic_infos = repo
        .run(&ListComicInfos {
            spec: &combined_filter_spec,
        })
        .await
        .unwrap();

    assert_eq!(combined_comic_infos.len(), 1);
    assert_eq!(
        combined_comic_infos.as_slice().first().unwrap().id,
        comic_fixture.comic_entry.id
    );

    let no_stage_filter_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: None,
        stages: Some(StageMask::try_filter_from(0xFFF).unwrap()),
        status: None,
        incl_opt: Vec::new(),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    assert_eq!(
        repo.run(&ListComicInfos {
            spec: &no_stage_filter_spec,
        })
        .await
        .unwrap()
        .len(),
        1,
    );
}

/// Verifies comic roundtrip via testcontainers.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn comic_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    verify_related_workset_team(&repo, &comic_fixture).await;

    verify_updated_search(&repo, &comic_fixture).await;

    verify_literal_search(&repo, &comic_fixture).await;

    let chapter_entry = ChapterEntry {
        id: format!("{}stage-chapter", PREFIX),
        comic_id: comic_fixture.comic_entry.id.clone().into(),
        is_pinned: true,
        index: 0,
        subtitle: "Stage Chapter".into(),
        creator_id: comic_fixture.creator_form.id.clone().into(),
    };

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        repo.step(
            context,
            &CreateChapter {
                entry: &chapter_entry,
            },
        )
        .await?;

        Ok::<(), crate::result::BaseError>(())
    })
    .await
    .unwrap();

    seed_sibling_stages(&repo, &nucl, &comic_fixture).await;

    verify_single_stage_filters(&repo, &nucl, &comic_fixture, &chapter_entry)
        .await;

    verify_combined_stage_filters(&repo, &nucl, &comic_fixture, &chapter_entry)
        .await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

fn single_stage_filter(stage: Stage, phase: StagePhase) -> StageMask {
    stage_filter(&[(stage, phase)])
}

fn stage_filter(stages: &[(Stage, StagePhase)]) -> StageMask {
    let value = stages.iter().fold(0xFFF, |value, (stage, phase)| {
        let shift = match stage {
            Stage::RawProvide => 0,
            Stage::Translate => 2,
            Stage::Proofread => 4,
            Stage::TypesetRedraw => 6,
            Stage::Review => 8,
            Stage::Publish => 10,
        };

        let phase = match phase {
            StagePhase::Pending => 0,
            StagePhase::Active => 1,
            StagePhase::Completed => 2,
        };

        (value & !(0b11 << shift)) | (phase << shift)
    });

    StageMask::try_filter_from(value).unwrap()
}
