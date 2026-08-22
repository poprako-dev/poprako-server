// termbase_unique_and_query_roundtrip(CreateTermbase, ListTermbaseInfos, UpdateTermbaseTermCount)(positive): termbase storage preserves normalized uniqueness while supporting escaped fuzzy search and atomic counts.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::read::spec::termbase::TermbaseListSpec;
use crate::model::write::termbase::TermbaseEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::comic::CreateComic;
use crate::part::repo::oper::termbase::{
    CreateTermbase, GetTermbaseInfo, ListTermbaseInfos, UpdateTermbaseTermCount,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::shared::RdbCore;

const PREFIX: &str = "rdb-test-termbase-domain-";

async fn create_termbase(
    repo: &HybRepo,
    nucl: &RdbNucl,
    termbase_entry: &TermbaseEntry,
) -> TermbaseInfo {
    nucl.coord(async |context| {
        repo.step(
            context,
            &CreateTermbase {
                entry: termbase_entry,
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap()
}

/// Verifies termbase unique and query roundtrip.
pub async fn termbase_unique_and_query_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: format!("{}main", PREFIX),
        team_id: Some(comic_fixture.team_entry.id.clone()),
        comic_id: None,
        name: "100%_Glossary".into(),
        description: Some("not searchable".into()),
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let created = create_termbase(&repo, &nucl, &termbase_entry).await;

    assert_eq!(created.term_count, 0);

    let comic_termbase_entry = TermbaseEntry {
        id: format!("{}comic-base", PREFIX),
        team_id: None,
        comic_id: Some(comic_fixture.comic_entry.id.clone()),
        name: "Comic Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    create_termbase(&repo, &nucl, &comic_termbase_entry).await;

    let mut sibling_comic_entry = test_shared::form::comic_entry(
        &format!("{}sibling-", PREFIX),
        &comic_fixture.workset_entry,
        &comic_fixture.creator_form,
    );

    sibling_comic_entry.index = 1;

    nucl.coord(async |context| {
        repo.step(
            context,
            &CreateComic {
                entry: &sibling_comic_entry,
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let sibling_termbase_entry = TermbaseEntry {
        id: format!("{}sibling-base", PREFIX),
        team_id: None,
        comic_id: Some(sibling_comic_entry.id.clone()),
        name: "Sibling Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    create_termbase(&repo, &nucl, &sibling_termbase_entry).await;

    let comic_list_spec = TermbaseListSpec::Comic {
        comic_id: comic_fixture.comic_entry.id.clone(),
        fuzzy_name: None,
        offset: 0,
        limit: 10,
    };

    let comic_termbases = repo
        .run(&ListTermbaseInfos {
            spec: &comic_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(comic_termbases.len(), 2);

    assert!(
        comic_termbases
            .iter()
            .any(|termbase_info| termbase_info.id == termbase_entry.id)
    );

    assert!(
        comic_termbases
            .iter()
            .any(|termbase_info| termbase_info.id == comic_termbase_entry.id)
    );

    assert!(
        comic_termbases
            .iter()
            .all(|termbase_info| termbase_info.id != sibling_termbase_entry.id)
    );

    let list_spec = TermbaseListSpec::Team {
        team_id: comic_fixture.team_entry.id.clone(),
        fuzzy_name: Some("%_G".into()),
        offset: 0,
        limit: 10,
    };

    let listed = repo
        .run(&ListTermbaseInfos { spec: &list_spec })
        .await
        .ok()
        .unwrap();

    assert_eq!(listed.len(), 1);

    nucl.coord(async |context| {
        repo.step(
            context,
            &UpdateTermbaseTermCount {
                id: &termbase_entry.id,
                delta: 1,
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let counted = repo
        .run(&GetTermbaseInfo {
            id: &termbase_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(counted.term_count, 1);

    let duplicate_entry = TermbaseEntry {
        id: format!("{}duplicate", PREFIX),
        name: " 100%_GLOSSARY ".into(),
        ..termbase_entry.clone()
    };

    let duplicate_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &CreateTermbase {
                    entry: &duplicate_entry,
                },
            )
            .await
        })
        .await;

    assert!(duplicate_result.is_err());

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
