// comic_roundtrip_reads_test_database_url(ComicStep)(positive): comic repo persists, lists by one-based display index, and refreshes composed search after update.

use super::*;

use crate::model::comic::{ComicInfoUpdate, ComicListKind, ComicListSpec};
use crate::part::repo::step::comic::ComicStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::value::comic::ComicInclOpt;

const PREFIX: &str = "rdb-test-comic-domain-";

#[tokio::test]
async fn comic_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_form.id.clone(),
        fuzzy_title: Some("Comic".into()),
        kind: ComicListKind::All,
        incl_opt: vec![ComicInclOpt::WorksetTeam],
        offset: 0,
        limit: 10,
    };

    let comic_infos =
        Execute::execute(&repo, &ComicStep::list_infos(&comic_list_spec))
            .await
            .ok()
            .unwrap();

    assert_eq!(comic_infos.len(), 1);
    assert_eq!(
        comic_infos[0].workset.as_ref().unwrap().id,
        comic_fixture.workset_form.id
    );
    assert_eq!(
        comic_infos[0].team.as_ref().unwrap().id,
        comic_fixture.team_form.id
    );

    let comic_info_update = ComicInfoUpdate {
        id: comic_fixture.comic_form.id.clone(),
        title: "RDB Comic Updated".into(),
        author: "RDB Author Updated".into(),
        description: Some("updated".into()),
    };

    Execute::execute(&repo, &ComicStep::update_info(&comic_info_update))
        .await
        .ok()
        .unwrap();

    let comic_info = Execute::execute(
        &repo,
        &ComicStep::get_info_by_id(&comic_fixture.comic_form.id, &[]),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(comic_info.title, "RDB Comic Updated");

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_form.id.clone(),
        fuzzy_title: Some("RDB Author Updated".into()),
        kind: ComicListKind::All,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
    };

    let comic_infos =
        Execute::execute(&repo, &ComicStep::list_infos(&comic_list_spec))
            .await
            .ok()
            .unwrap();

    assert_eq!(comic_infos.len(), 1);
    assert_eq!(comic_infos[0].id, comic_fixture.comic_form.id);

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_form.id.clone(),
        fuzzy_title: Some("1".into()),
        kind: ComicListKind::All,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
    };

    let comic_infos =
        Execute::execute(&repo, &ComicStep::list_infos(&comic_list_spec))
            .await
            .ok()
            .unwrap();

    assert_eq!(comic_infos.len(), 1);
    assert_eq!(comic_infos[0].index, 0);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
