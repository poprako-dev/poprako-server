// term_array_unique_and_fuzzy_roundtrip(CreateTerm, ListTermInfos, UpsertTerms)(positive): term storage preserves target order, applies bounded import upserts, rejects normalized duplicate sources, and treats fuzzy wildcard characters literally.

use super::*;

use poprako_orchestra::Nucl as _;

use poprako_rdb_core::RdbCore;

use crate::model::write::term::{TermEntry, TermRepl};
use crate::model::write::termbase::TermbaseEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::term::{
    CreateTerm, GetTermInfo, ListTermInfos, UpsertTerms,
};
use crate::part::repo::oper::termbase::CreateTermbase;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;

const PREFIX: &str = "rdb-test-term-domain-";

/// Verifies term array unique and fuzzy roundtrip.
pub async fn term_array_unique_and_fuzzy_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: format!("{}base", PREFIX),
        team_id: None,
        comic_id: Some(comic_fixture.comic_entry.id.clone()),
        name: "Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateTermbase {
                entry: &termbase_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let term_entry = TermEntry {
        id: format!("{}main", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "100%_Hero".into(),
        targets: vec!["勇者".into(), "英雄".into()],
        comment: Some("not searchable".into()),
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    nucl.coord(async |context| {
        //
        repo.step(&mut *context, &CreateTerm { entry: &term_entry })
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let persisted = repo
        .run(&GetTermInfo { id: &term_entry.id })
        .await
        .ok()
        .unwrap();

    assert_eq!(persisted.targets, vec!["勇者", "英雄"]);

    let listed = repo
        .run(&ListTermInfos::Query {
            termbase_id: &termbase_entry.id,
            fuzzy_source: Some("%_H"),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(listed.len(), 1);

    let imported_entry = TermEntry {
        id: format!("{}imported", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "Alpha".into(),
        targets: vec!["阿尔法".into()],
        comment: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let imported_update = TermRepl {
        id: term_entry.id.clone(),
        source: term_entry.source.clone(),
        targets: vec!["勇者".into(), "英雄".into(), "主角".into()],
        comment: Some("imported".into()),
    };

    nucl.coord(async |context| {
        repo.step(
            context,
            &UpsertTerms {
                entries: std::slice::from_ref(&imported_entry),
                updates: std::slice::from_ref(&imported_update),
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let port_terms = nucl
        .coord(async |context| {
            repo.step(
                context,
                &ListTermInfos::Termbase {
                    termbase_id: &termbase_entry.id,
                },
            )
            .await
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(port_terms.len(), 2);

    assert_eq!(port_terms[0].id, term_entry.id);

    assert_eq!(port_terms[0].targets, ["勇者", "英雄", "主角"]);

    assert_eq!(port_terms[1].id, imported_entry.id);

    let duplicate_entry = TermEntry {
        id: format!("{}duplicate", PREFIX),
        source: " 100%_HERO ".into(),
        ..term_entry.clone()
    };

    let duplicate_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &CreateTerm {
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
