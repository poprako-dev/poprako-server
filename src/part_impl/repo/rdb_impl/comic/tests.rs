// comic_roundtrip_uses_testcontainer(ComicRepo)(positive): comic repo persists, lists by one-based display index, and refreshes composed search after update.

use poprako_orchestra::Run as _;

use poprako_rdb_core::RdbCore;

use crate::model::read::spec::comic::ComicListSpec;
use crate::model::write::comic::ComicRepl;
use crate::part::repo::oper::comic::{
    GetComicInfo, ListComicInfos, UpdateComic,
};
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
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

    let comic_list_spec = ComicListSpec {
        workset_id: comic_fixture.workset_entry.id.clone(),
        fuzzy_title: Some("1".into()),
        stages: None,
        status: None,
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

    for fuzzy_title in ["%", "_", "\\"] {
        //
        let comic_list_spec = ComicListSpec {
            workset_id: comic_fixture.workset_entry.id.clone(),
            fuzzy_title: Some(fuzzy_title.into()),
            stages: None,
            status: None,
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
    }

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
