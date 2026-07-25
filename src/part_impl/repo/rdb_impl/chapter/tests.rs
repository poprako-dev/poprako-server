// chapter_roundtrip_uses_testcontainer(ChapterRepo)(positive): chapter repo persists, lists, and finds pinned chapter rows in an isolated PostgreSQL container.

use poprako_orchestra::{Nucl, Run as _, Step as _};

use crate::model::chapter::{ChapterInfoListSpec, ChapterStageUpdate};
use crate::part::repo::oper::chapter::{FindPinnedChapterInfo, GetChapterInfo, ListChapterInfos, StartChapterStage, UpdateChapterStage};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::accept;
use crate::value::chapter::{ChapterInclOpt, Stage, StageMask, StagePhase};

const PREFIX: &str = "rdb-test-chapter-domain-";

/// Verifies chapter roundtrip via testcontainers.
/// Verifies chapter roundtrip via testcontainers.
pub async fn chapter_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let stage_mask = StageMask::try_from(0u32).ok().unwrap();

    let chapter_stage_update = ChapterStageUpdate {
        id: chapter_fixture.chapter_entry.id.clone(),
        stages: stage_mask,
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &UpdateChapterStage {
                    update: &chapter_stage_update,
                },
            )
            .await?;

            accept(())
        })
        .await
        .ok()
        .unwrap();

    let first_start = repo
        .run(&StartChapterStage {
            id: &chapter_fixture.chapter_entry.id,
            stage: Stage::Translate,
        })
        .await
        .ok()
        .unwrap();

    let repeated_start = repo
        .run(&StartChapterStage {
            id: &chapter_fixture.chapter_entry.id,
            stage: Stage::Translate,
        })
        .await
        .ok()
        .unwrap();

    assert!(first_start);

    assert!(!repeated_start);

    let started_chapter = repo
        .run(&GetChapterInfo {
            id: &chapter_fixture.chapter_entry.id,
            incls: &[],
        })
        .await
        .ok()
        .unwrap();

    assert!(
        started_chapter
            .stages
            .has_phase(Stage::Translate, StagePhase::Active)
    );

    let chapter_list_spec = ChapterInfoListSpec {
        comic_id: chapter_fixture.comic_entry.id.clone(),
        incl_opt: vec![ChapterInclOpt::Creator],
        offset: 0,
        limit: 10,
    };

    let chapter_infos = repo
        .run(&ListChapterInfos {
            spec: &chapter_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(chapter_infos.len(), 1);

    assert_eq!(
        chapter_infos[0].creator.as_ref().unwrap().id,
        chapter_fixture.creator_form.id
    );

    let chapter_list_spec = ChapterInfoListSpec {
        comic_id: chapter_fixture.comic_entry.id.clone(),
        incl_opt: vec![ChapterInclOpt::ComicWorksetTeam],
        offset: 0,
        limit: 10,
    };

    let chapter_infos = repo
        .run(&ListChapterInfos {
            spec: &chapter_list_spec,
        })
        .await
        .ok()
        .unwrap();

    let comic_info = chapter_infos[0].comic.as_ref().unwrap();

    assert_eq!(comic_info.id, chapter_fixture.comic_entry.id);

    assert_eq!(
        comic_info.workset.as_ref().unwrap().id,
        chapter_fixture.workset_entry.id
    );

    assert_eq!(
        comic_info.team.as_ref().unwrap().id,
        chapter_fixture.team_entry.id
    );

    let pinned_chapter_info = repo
        .run(&FindPinnedChapterInfo {
            comic_id: &chapter_fixture.comic_entry.id,
            incls: &[ChapterInclOpt::Creator],
        })
        .await
        .ok()
        .unwrap()
        .unwrap();

    assert_eq!(pinned_chapter_info.id, chapter_fixture.chapter_entry.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
