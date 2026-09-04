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
                termbase_id: &termbase_entry.id,
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

    let bulk_entries = (0..200)
        .map(|index| TermEntry {
            id: format!("{}bulk-{index:03}", PREFIX),
            termbase_id: termbase_entry.id.clone(),
            source: format!("Bulk {index:03}"),
            targets: vec![format!("Target {index:03}")],
            comment: None,
            creator_id: comic_fixture.creator_form.id.clone(),
        })
        .collect::<Vec<_>>();

    nucl.coord(async |context| {
        repo.step(
            context,
            &UpsertTerms {
                termbase_id: &termbase_entry.id,
                entries: &bulk_entries,
                updates: &[],
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let bulk_updates = bulk_entries
        .iter()
        .enumerate()
        .map(|(index, entry)| TermRepl {
            id: entry.id.clone(),
            source: format!("{} updated", entry.source),
            targets: vec![format!("Target {index:03} updated")],
            comment: Some("batch updated".into()),
        })
        .collect::<Vec<_>>();

    nucl.coord(async |context| {
        repo.step(
            context,
            &UpsertTerms {
                termbase_id: &termbase_entry.id,
                entries: &[],
                updates: &bulk_updates,
            },
        )
        .await
    })
    .await
    .ok()
    .unwrap();

    let bulk_terms = nucl
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
        .unwrap()
        .into_iter()
        .filter(|term_info| term_info.id.contains("bulk-"))
        .collect::<Vec<_>>();

    assert_eq!(bulk_terms.len(), 200);
    assert!(bulk_terms.iter().all(|term_info| {
        term_info.source.ends_with(" updated")
            && term_info.comment.as_deref() == Some("batch updated")
    }));
    let Some(first_bulk_term) = bulk_terms.as_slice().first() else {
        panic!("bulk terms must not be empty");
    };

    assert!(bulk_terms.iter().all(|term_info| {
        term_info.updated_at == first_bulk_term.updated_at
    }));

    let other_termbase_entry = TermbaseEntry {
        id: format!("{}other-base", PREFIX),
        name: "Other Glossary".into(),
        ..termbase_entry.clone()
    };

    let other_term_entry = TermEntry {
        id: format!("{}other-term", PREFIX),
        termbase_id: other_termbase_entry.id.clone(),
        source: "Other".into(),
        targets: vec!["其他".into()],
        comment: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    nucl.coord(async |context| {
        repo.step(
            &mut *context,
            &CreateTermbase {
                entry: &other_termbase_entry,
            },
        )
        .await?;

        repo.step(
            context,
            &CreateTerm {
                entry: &other_term_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let rollback_entry = TermEntry {
        id: format!("{}rollback", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "Rollback".into(),
        targets: vec!["回滚".into()],
        comment: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let wrong_scope_update = TermRepl {
        id: other_term_entry.id.clone(),
        source: "Other updated".into(),
        targets: vec!["其他更新".into()],
        comment: Some("must roll back".into()),
    };

    let wrong_scope_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &UpsertTerms {
                    termbase_id: &termbase_entry.id,
                    entries: std::slice::from_ref(&rollback_entry),
                    updates: std::slice::from_ref(&wrong_scope_update),
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from);

    assert!(matches!(
        wrong_scope_result,
        Err(BaseError::Unrecoverable { .. })
    ));

    assert!(
        repo.run(&GetTermInfo {
            id: &rollback_entry.id,
        })
        .await
        .is_err()
    );

    let unchanged_other = repo
        .run(&GetTermInfo {
            id: &other_term_entry.id,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(unchanged_other.source, other_term_entry.source);

    let Some(duplicate_update) = bulk_updates.as_slice().first().cloned()
    else {
        panic!("bulk updates must not be empty");
    };

    let duplicate_result = nucl
        .coord(async |context| {
            repo.step(
                context,
                &UpsertTerms {
                    termbase_id: &termbase_entry.id,
                    entries: &[],
                    updates: &[duplicate_update.clone(), duplicate_update],
                },
            )
            .await
        })
        .await
        .map_err(BaseError::from);

    assert!(matches!(
        duplicate_result,
        Err(BaseError::Unrecoverable { .. })
    ));

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
