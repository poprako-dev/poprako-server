// term_array_unique_and_fuzzy_roundtrip(CreateTerm, ListTermInfos)(positive): term storage preserves target order, rejects normalized duplicate sources, and treats fuzzy wildcard characters literally.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::term::{TermEntry, TermInfoListSpec};
use crate::model::termbase::TermbaseEntry;
use crate::part::repo::oper::term::{CreateTerm, GetTermInfo, ListTermInfos};
use crate::part::repo::oper::termbase::CreateTermbase;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::part_impl::shared::RdbCore;
use crate::result::BaseError;

const PREFIX: &str = "rdb-test-term-domain-";

/// Verifies term array unique and fuzzy roundtrip.
pub async fn term_array_unique_and_fuzzy_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: format!("{}base", PREFIX),
        team_id: None,
        comic_id: Some(comic_fixture.comic_entry.id.clone()),
        name: "Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    drive
        .coord(async |context| {
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

    drive
        .coord(async |context| {
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

    let list_spec = TermInfoListSpec {
        termbase_id: termbase_entry.id.clone(),
        fuzzy_source: Some("%_H".into()),
        offset: 0,
        limit: 10,
    };

    let listed = repo
        .run(&ListTermInfos { spec: &list_spec })
        .await
        .ok()
        .unwrap();

    assert_eq!(listed.len(), 1);

    let duplicate_entry = TermEntry {
        id: format!("{}duplicate", PREFIX),
        source: " 100%_HERO ".into(),
        ..term_entry.clone()
    };

    let duplicate_result = drive
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
