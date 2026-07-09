// chapter_roundtrip_reads_test_database_url(ChapterStep)(positive): chapter repo persists, lists, and finds pinned chapter rows in the local test database.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::chapter::{ChapterListSpec, ChapterStageUpdate};
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::chapter::{ChapterInclOpt, StageMask};

const PREFIX: &str = "rdb-test-chapter-domain-";

#[tokio::test]
async fn chapter_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let chapter_fixture = test_shared::seed_chapter(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let stage_mask = StageMask::try_from(0u32).ok().unwrap();

    let chapter_stage_update = ChapterStageUpdate {
        id: chapter_fixture.chapter_form.id.clone(),
        stages: stage_mask,
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &ChapterStep::update_stage(&chapter_stage_update),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let chapter_list_spec = ChapterListSpec {
        comic_id: chapter_fixture.comic_form.id.clone(),
        incl_opt: vec![ChapterInclOpt::Creator],
        offset: 0,
        limit: 10,
    };

    let chapter_infos =
        Execute::execute(&repo, &ChapterStep::list_infos(&chapter_list_spec))
            .await
            .ok()
            .unwrap();

    assert_eq!(chapter_infos.len(), 1);
    assert_eq!(
        chapter_infos[0].creator.as_ref().unwrap().id,
        chapter_fixture.creator_form.id
    );

    let chapter_list_spec = ChapterListSpec {
        comic_id: chapter_fixture.comic_form.id.clone(),
        incl_opt: vec![ChapterInclOpt::ComicWorksetTeam],
        offset: 0,
        limit: 10,
    };

    let chapter_infos =
        Execute::execute(&repo, &ChapterStep::list_infos(&chapter_list_spec))
            .await
            .ok()
            .unwrap();

    let comic_info = chapter_infos[0].comic.as_ref().unwrap();

    assert_eq!(comic_info.id, chapter_fixture.comic_form.id);
    assert_eq!(
        comic_info.workset.as_ref().unwrap().id,
        chapter_fixture.workset_form.id
    );
    assert_eq!(
        comic_info.team.as_ref().unwrap().id,
        chapter_fixture.team_form.id
    );

    let pinned_chapter_info = Execute::execute(
        &repo,
        &ChapterStep::find_pinned_info_by_comic_id(
            &chapter_fixture.comic_form.id,
            &[ChapterInclOpt::Creator],
        ),
    )
    .await
    .ok()
    .unwrap()
    .unwrap();

    assert_eq!(pinned_chapter_info.id, chapter_fixture.chapter_form.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
