#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// term_array_unique_and_fuzzy_roundtrip(CreateTerm, ListTermInfos, UpsertTerms)(positive): term storage preserves target order, applies bounded import upserts, rejects normalized duplicate sources, and treats fuzzy wildcard characters literally.

use super::*;

use poprako_orchestra::Nucl as _;

use poprako_rdb_core::RdbCore;

use crate::config::cache::CacheConfig;
use crate::config::repo::RepoConfig;
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
use crate::part_impl::repo::rdb_impl::test_shared::ComicFixture;
use crate::result::BaseError;

const PREFIX: &str = "rdb-test-term-domain-";

// Verify a mixed insert and update preserves target order in the import listing.
// Exercise multi-chunk inserts and updates with one shared modification timestamp.
// Verify an update in another terminology base rolls back the accompanying insert.
// Reject duplicate identifiers within one update batch.
// Reject sources that differ only by case and surrounding whitespace.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_normalized_duplicate(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    term_entry: &TermEntry<'_>,
) {
    let duplicate_entry = TermEntry {
        id: format!("{}duplicate", PREFIX),
        source: " 100%_HERO ",
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
}

async fn verify_duplicate_updates(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    termbase_entry: &TermbaseEntry<'_>,
    bulk_updates: &[TermRepl<'_>],
) {
    let duplicate_update = bulk_updates.first().cloned().unwrap();

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
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_wrong_scope_rollback(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    termbase_entry: &TermbaseEntry<'_>,
) {
    let other_termbase_entry = TermbaseEntry {
        id: format!("{}other-base", PREFIX),
        name: "Other Glossary",
        ..termbase_entry.clone()
    };

    let other_term_entry = TermEntry {
        id: format!("{}other-term", PREFIX),
        termbase_id: other_termbase_entry.id.clone(),
        source: "Other",
        targets: vec!["其他"],
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
    .unwrap();

    let rollback_entry = TermEntry {
        id: format!("{}rollback", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "Rollback",
        targets: vec!["回滚"],
        comment: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let wrong_scope_update = TermRepl {
        id: other_term_entry.id.clone(),
        source: "Other updated",
        targets: vec!["其他更新"],
        comment: Some("must roll back"),
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
        .unwrap();

    assert_eq!(unchanged_other.source, other_term_entry.source);
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_bulk_upsert(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    termbase_entry: &TermbaseEntry<'_>,
) -> Vec<crate::model::read::proj::term::TermInfo> {
    let sources = (0..200)
        .map(|index| format!("Bulk {index:03}"))
        .collect::<Vec<_>>();

    let targets = (0..200)
        .map(|index| format!("Target {index:03}"))
        .collect::<Vec<_>>();

    let bulk_entries = sources
        .iter()
        .zip(&targets)
        .enumerate()
        .map(|(index, (source, target))| TermEntry {
            id: format!("{}bulk-{index:03}", PREFIX),
            termbase_id: termbase_entry.id.clone(),
            source,
            targets: vec![target],
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
    .unwrap();

    let updated_sources = sources
        .iter()
        .map(|source| format!("{source} updated"))
        .collect::<Vec<_>>();

    let updated_targets = targets
        .iter()
        .map(|target| format!("{target} updated"))
        .collect::<Vec<_>>();

    let bulk_updates = bulk_entries
        .iter()
        .zip(updated_sources.iter().zip(&updated_targets))
        .map(|(entry, (source, target))| TermRepl {
            id: entry.id.clone(),
            source,
            targets: vec![target],
            comment: Some("batch updated"),
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
        .unwrap()
        .into_iter()
        .filter(|term_info| term_info.id.contains("bulk-"))
        .collect::<Vec<_>>();

    assert_eq!(bulk_terms.len(), 200);
    assert!(bulk_terms.iter().all(|term_info| {
        term_info.source.ends_with(" updated")
            && term_info.comment.as_deref() == Some("batch updated")
    }));
    let first_bulk_term = bulk_terms.as_slice().first().unwrap();

    assert!(bulk_terms.iter().all(|term_info| {
        term_info.updated_at == first_bulk_term.updated_at
    }));
    bulk_terms
}

#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
async fn verify_import_upsert(
    repo: &HybRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    termbase_entry: &TermbaseEntry<'_>,
    term_entry: &TermEntry<'_>,
) {
    let imported_entry = TermEntry {
        id: format!("{}imported", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "Alpha",
        targets: vec!["阿尔法"],
        comment: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let imported_update = TermRepl {
        id: term_entry.id.clone(),
        source: term_entry.source,
        targets: vec!["勇者", "英雄", "主角"],
        comment: Some("imported"),
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
        .unwrap();

    assert_eq!(port_terms.len(), 2);

    assert_eq!(port_terms.as_slice().first().unwrap().id, term_entry.id);

    assert_eq!(
        port_terms.as_slice().first().unwrap().targets,
        ["勇者", "英雄", "主角"]
    );

    assert_eq!(port_terms.get(1).unwrap().id, imported_entry.id);
}

/// Verifies term array unique and fuzzy roundtrip.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
#[expect(
    clippy::uninlined_format_args,
    reason = "Repository formatting keeps interpolation arguments explicit"
)]
pub async fn term_array_unique_and_fuzzy_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = HybRepo::new(
        shared.clone(),
        CacheConfig::default(),
        RepoConfig::default(),
    );

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: format!("{}base", PREFIX),
        team_id: None,
        comic_id: Some(comic_fixture.comic_entry.id.clone()),
        name: "Glossary",
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
    .unwrap();

    let term_entry = TermEntry {
        id: format!("{}main", PREFIX),
        termbase_id: termbase_entry.id.clone(),
        source: "100%_Hero",
        targets: vec!["勇者", "英雄"],
        comment: Some("not searchable"),
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    nucl.coord(async |context| {
        //
        repo.step(&mut *context, &CreateTerm { entry: &term_entry })
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .unwrap();

    let persisted =
        repo.run(&GetTermInfo { id: &term_entry.id }).await.unwrap();

    assert_eq!(persisted.targets, vec!["勇者", "英雄"]);

    let listed = repo
        .run(&ListTermInfos::Query {
            termbase_id: &termbase_entry.id,
            fuzzy_source: Some("%_H"),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        })
        .await
        .unwrap();

    assert_eq!(listed.len(), 1);

    verify_import_upsert(
        &repo,
        &nucl,
        &comic_fixture,
        &termbase_entry,
        &term_entry,
    )
    .await;

    let bulk_terms =
        verify_bulk_upsert(&repo, &nucl, &comic_fixture, &termbase_entry).await;

    let bulk_updates = bulk_terms
        .iter()
        .map(|term_info| TermRepl {
            id: term_info.id.clone(),
            source: &term_info.source,
            targets: term_info.targets.iter().map(String::as_str).collect(),
            comment: term_info.comment.as_deref(),
        })
        .collect::<Vec<_>>();

    verify_wrong_scope_rollback(&repo, &nucl, &comic_fixture, &termbase_entry)
        .await;

    verify_duplicate_updates(&repo, &nucl, &termbase_entry, &bulk_updates)
        .await;

    verify_normalized_duplicate(&repo, &nucl, &term_entry).await;

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}
