// termbase_constraints_and_query_roundtrip(CreateTermbase, ListTermbaseInfos, UpdateTermbaseTermCount)(positive): termbase storage enforces scope and normalized uniqueness while supporting escaped fuzzy search and atomic counts.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::termbase::{TermbaseEntry, TermbaseInfoListSpec};
use crate::part::repo::oper::termbase::{CreateTermbase, GetTermbaseInfo, ListTermbaseInfos, UpdateTermbaseTermCount};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;

const PREFIX: &str = "rdb-test-termbase-domain-";

async fn create_termbase(
    repo: &RdbRepo,
    drive: &RdbDrive,
    termbase_entry: &TermbaseEntry,
) -> TermbaseInfo {
    drive
        .coord(async |context| {
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

/// Verifies termbase constraints and query roundtrip.
/// Verifies termbase constraints and query roundtrip.
pub async fn termbase_constraints_and_query_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: format!("{}main", PREFIX),
        team_id: Some(comic_fixture.team_entry.id.clone()),
        comic_id: None,
        name: "100%_Glossary".into(),
        description: Some("not searchable".into()),
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let created = create_termbase(&repo, &drive, &termbase_entry).await;

    assert_eq!(created.term_count, 0);

    let list_spec = TermbaseInfoListSpec::Team {
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

    drive
        .coord(async |context| {
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

    let duplicate_result = drive
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

    let invalid_scope_entry = TermbaseEntry {
        id: format!("{}invalid-scope", PREFIX),
        team_id: Some(comic_fixture.team_entry.id.clone()),
        comic_id: Some(comic_fixture.comic_entry.id.clone()),
        name: "Invalid scope".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let invalid_scope_result = drive
        .coord(async |context| {
            repo.step(
                context,
                &CreateTermbase {
                    entry: &invalid_scope_entry,
                },
            )
            .await
        })
        .await;

    assert!(invalid_scope_result.is_err());

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
