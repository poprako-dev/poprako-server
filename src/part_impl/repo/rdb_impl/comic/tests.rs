// comic_roundtrip_uses_testcontainer(ComicRepo)(positive): comic repo persists, lists by one-based display index, and refreshes composed search after update.

use poprako_orchestra::Run as _;

use crate::model::comic::{
    ComicInfoListKind, ComicInfoListSpec, ComicInfoUpdate,
};
use crate::part::repo::oper::comic::{
    GetComicInfo, ListComicInfos, UpdateComic,
};
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::value::comic::ComicInclOpt;

const PREFIX: &str = "rdb-test-comic-domain-";

pub async fn comic_roundtrip_uses_testcontainer(shared: RdbCore) {
    test_shared::reset(&shared, PREFIX).await;

    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let comic_list_spec = ComicInfoListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("Comic".into()),
        kind: ComicInfoListKind::All,
        incl_opt: vec![ComicInclOpt::WorksetTeam],
        offset: 0,
        limit: 10,
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

    let comic_info_update = ComicInfoUpdate {
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

    let comic_list_spec = ComicInfoListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("RDB Author Updated".into()),
        kind: ComicInfoListKind::All,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
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

    let comic_list_spec = ComicInfoListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("1".into()),
        kind: ComicInfoListKind::All,
        incl_opt: Vec::new(),
        offset: 0,
        limit: 10,
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

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
