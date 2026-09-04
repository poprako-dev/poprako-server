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
use crate::value::chapter::mask::StageMask;
use crate::value::chapter::stage::{Stage, StagePhase};
use crate::value::comic::ComicInclOpt;

const PREFIX: &str = "rdb-test-comic-domain-";

/// Verifies comic roundtrip via testcontainers.
/// Verifies comic roundtrip via testcontainers.
pub async fn comic_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

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
        .ok()
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(
        comic_infos[0].workset.as_ref().unwrap().id,
        comic_fixture.workset_entry.id
    );

    assert_eq!(
        comic_infos[0].team.as_ref().unwrap().id,
        comic_fixture.team_entry.id
    );

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
    .ok()
    .unwrap();

    let comic_info = repo
        .run(&GetComicInfo {
            id: &comic_fixture.comic_entry.id,
            incls: &[],
        })
        .await
        .ok()
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
        .ok()
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(comic_infos[0].id, comic_fixture.comic_entry.id);

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
        .ok()
        .unwrap();

    assert_eq!(comic_infos.len(), 1);

    assert_eq!(comic_infos[0].index, 0);

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
            .ok()
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
    .ok()
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
            .ok()
            .unwrap();

        assert_eq!(comic_infos.len(), 1);
    }

    let chapter_entry = ChapterEntry {
        id: format!("{}stage-chapter", PREFIX),
        comic_id: comic_fixture.comic_entry.id.clone(),
        is_pinned: true,
        index: 0,
        subtitle: "Stage Chapter".into(),
        creator_id: comic_fixture.creator_form.id.clone(),
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
    .ok()
    .unwrap();

    let sibling_workset = WorksetEntry {
        id: format!("{}sibling-workset", PREFIX),
        team_id: comic_fixture.team_entry.id.clone(),
        index: 1,
        name: "Sibling Workset".into(),
        description: None,
    };

    let sibling_comic = ComicEntry {
        id: format!("{}sibling-comic", PREFIX),
        workset_id: sibling_workset.id.clone(),
        index: 0,
        title: "Sibling Comic".into(),
        author: "Sibling Author".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let sibling_chapter = ChapterEntry {
        id: format!("{}sibling-chapter", PREFIX),
        comic_id: sibling_comic.id.clone(),
        is_pinned: true,
        index: 0,
        subtitle: "Sibling Chapter".into(),
        creator_id: comic_fixture.creator_form.id.clone(),
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
    .ok()
    .unwrap();

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
        .ok()
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
            .ok()
            .unwrap();

        assert_eq!(comic_infos.len(), 1, "{stage:?} {phase:?}");
    }

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
    .ok()
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
        .ok()
        .unwrap();

    assert_eq!(combined_comic_infos.len(), 1);
    assert_eq!(combined_comic_infos[0].id, comic_fixture.comic_entry.id);

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
        .ok()
        .unwrap()
        .len(),
        1,
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
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
