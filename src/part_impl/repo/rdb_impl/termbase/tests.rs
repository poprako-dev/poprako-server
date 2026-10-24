#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

// termbase_unique_and_query_roundtrip(CreateTermbase, ListTermbaseInfos, UpdateTermbaseTermCount)(positive): termbase storage preserves normalized uniqueness while supporting escaped fuzzy search and atomic counts.

use super::*;

use poprako_orchestra::{Nucl as _, OperRun as _, OperStep as _};

use crate::config::repo::RepoConfig;
use crate::data::instr::termbase_port::{
    AppendTermbaseInstr, AppendTermbaseTermInstr,
};
use crate::model::read::spec::termbase::TermbaseListSpec;
use crate::model::shared::user::UserToken;
use crate::model::write::member::MemberEntry;
use crate::model::write::termbase::TermbaseEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::comic::CreateComic;
use crate::part::repo::oper::member::CreateMember;
use crate::part::repo::oper::term::ListTermInfos;
use crate::part::repo::oper::termbase::{
    CreateTermbase, GetTermbaseInfo, ListTermbaseInfos, UpdateTermbaseTermCount,
};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::RdbRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::part_impl::repo::rdb_impl::test_shared::ComicFixture;
use crate::usecase::termbase_port as termbase_port_usecase;
use crate::value::role::{RoleField, RoleMask};
use poprako_rdb_core::RdbCore;

const PREFIX: &str = "rdb-test-termbase-domain-";

// Verify comic listings include team and owning-comic bases while excluding siblings.
async fn verify_comic_termbase_scope(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    comic_fixture: &ComicFixture,
    termbase_entry: &TermbaseEntry,
) {
    let comic_termbase_entry = TermbaseEntry {
        id: [PREFIX, "comic-base"].concat(),
        team_id: None,
        comic_id: Some(comic_fixture.comic_info.id.clone()),
        name: "Comic Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    create_termbase(repo, nucl, &comic_termbase_entry).await;

    let mut sibling_comic_entry = test_shared::form::comic_entry(
        &[PREFIX, "sibling-"].concat(),
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
    .unwrap();

    let sibling_termbase_entry = TermbaseEntry {
        id: [PREFIX, "sibling-base"].concat(),
        team_id: None,
        comic_id: Some(sibling_comic_entry.id.clone()),
        name: "Sibling Glossary".into(),
        description: None,
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    create_termbase(repo, nucl, &sibling_termbase_entry).await;

    let comic_list_spec = TermbaseListSpec::Comic {
        comic_id: comic_fixture.comic_info.id.clone(),
        fuzzy_name: None,
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let comic_termbases = repo
        .run(&ListTermbaseInfos {
            spec: &comic_list_spec,
        })
        .await
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
}

async fn create_termbase(
    repo: &RdbRepo,
    nucl: &RdbNucl,
    entry: &TermbaseEntry,
) -> TermbaseInfo {
    nucl.coord(async |context| {
        repo.step(context, &CreateTermbase { entry }).await
    })
    .await
    .unwrap()
}

/// Verifies termbase unique and query roundtrip.
/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn termbase_unique_and_query_roundtrip(shared: RdbCore) {
    //
    let comic_fixture = test_shared::seed_comic(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let termbase_entry = TermbaseEntry {
        id: [PREFIX, "main"].concat(),
        team_id: Some(comic_fixture.team_entry.id.clone()),
        comic_id: None,
        name: "100%_Glossary".into(),
        description: Some("not searchable".into()),
        creator_id: comic_fixture.creator_form.id.clone(),
    };

    let created = create_termbase(&repo, &nucl, &termbase_entry).await;

    assert_eq!(created.term_count, 0);

    verify_comic_termbase_scope(&repo, &nucl, &comic_fixture, &termbase_entry)
        .await;

    let list_spec = TermbaseListSpec::Team {
        team_id: comic_fixture.team_entry.id.clone(),
        fuzzy_name: Some("%_G".into()),
        offset: 0,
        limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
    };

    let listed = repo
        .run(&ListTermbaseInfos { spec: &list_spec })
        .await
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
    .unwrap();

    let counted = repo
        .run(&GetTermbaseInfo {
            id: &termbase_entry.id,
        })
        .await
        .unwrap();

    assert_eq!(counted.term_count, 1);

    let duplicate_entry = TermbaseEntry {
        id: [PREFIX, "duplicate"].concat(),
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

    test_shared::cleanup(&shared, PREFIX).await.unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .unwrap();
}

// Build one ordinary append request for the database regression.
fn append_instr(
    source: &str,
    target: &str,
    comment: Option<&str>,
) -> AppendTermbaseInstr {
    AppendTermbaseInstr {
        terms: vec![AppendTermbaseTermInstr {
            source: source.into(),
            targets: vec![target.into()],
            comment: comment.map(str::to_owned),
        }],
    }
}

// Verify a persisted no-op preserves aggregate and entry timestamps.
async fn verify_append_noop(
    repo: &RdbRepo,
    nucl: &RdbNucl<ReptRead>,
    token: UserToken,
    base_id: &str,
) {
    let terms = ListTermInfos::Termbase {
        termbase_id: base_id,
    }
    .run_on(repo)
    .await
    .unwrap();

    let original_updated_at = terms.as_slice().first().unwrap().updated_at;

    let before = GetTermbaseInfo { id: base_id }.run_on(repo).await.unwrap();

    let val = termbase_port_usecase::append(
        (nucl, repo),
        token,
        base_id.into(),
        append_instr("SOURCE", "preferred", Some("Old")),
    )
    .await
    .unwrap();

    assert_eq!((val.added, val.updated, val.unchanged), (0, 0, 1));

    let after = GetTermbaseInfo { id: base_id }.run_on(repo).await.unwrap();

    assert_eq!(after.term_count, 1);

    assert_eq!(after.name, "Kept name");

    assert_eq!(after.description.as_deref(), Some("Kept description"));

    assert_eq!(after.updated_at, before.updated_at);

    let terms = ListTermInfos::Termbase {
        termbase_id: base_id,
    }
    .run_on(repo)
    .await
    .unwrap();

    assert_eq!(
        terms.as_slice().first().unwrap().updated_at,
        original_updated_at
    );
}

// append(append)(positive): PostgreSQL writes preserve identity and no-op timestamps under concurrent batches.
#[tokio::test]
async fn append_roundtrip_and_concurrent_merges() {
    let database = crate::shared::test_rdb::start().await;

    let shared = database.core();

    let fixture = test_shared::seed_user_and_team(&shared, "append-rdb-").await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared);

    let member_entry = MemberEntry {
        id: "append-rdb-member".into(),
        user_id: fixture.user_entry.id.clone(),
        user_nickname: fixture.user_entry.nickname.clone(),
        team_id: fixture.team_entry.id.clone(),
        roles: RoleMask::from(RoleField::TRANSLATOR),
    };

    nucl.coord(async |context| {
        CreateMember {
            entry: &member_entry,
        }
        .step_on(&repo, context)
        .await
    })
    .await
    .unwrap();

    let base_entry = TermbaseEntry {
        id: "append-rdb-base".into(),
        team_id: Some(fixture.team_entry.id),
        comic_id: None,
        name: "Kept name".into(),
        description: Some("Kept description".into()),
        creator_id: fixture.user_entry.id.clone(),
    };

    create_termbase(&repo, &nucl, &base_entry).await;

    let token = UserToken {
        user_id: fixture.user_entry.id,
    };

    let val = termbase_port_usecase::append(
        (&nucl, &repo),
        token.clone(),
        base_entry.id.clone(),
        append_instr("Source", "Preferred", Some("Old")),
    )
    .await
    .unwrap();

    assert_eq!((val.added, val.updated, val.unchanged), (1, 0, 0));

    let terms = ListTermInfos::Termbase {
        termbase_id: &base_entry.id,
    }
    .run_on(&repo)
    .await
    .unwrap();

    let original = terms.as_slice().first().unwrap();

    let left = append_instr(" source ", "Second", None);

    let right = append_instr("SOURCE", "Third", None);

    let (left_result, right_result) = tokio::join!(
        termbase_port_usecase::append(
            (&nucl, &repo),
            token.clone(),
            base_entry.id.clone(),
            left
        ),
        termbase_port_usecase::append(
            (&nucl, &repo),
            token.clone(),
            base_entry.id.clone(),
            right
        ),
    );

    assert!(left_result.is_ok() || right_result.is_ok());

    // Repeatable-read conflicts may reject one transaction; retry only the known failed batch.
    for (result, target) in [(left_result, "Second"), (right_result, "Third")] {
        if result.is_err() {
            termbase_port_usecase::append(
                (&nucl, &repo),
                token.clone(),
                base_entry.id.clone(),
                append_instr("source", target, None),
            )
            .await
            .unwrap();
        }
    }

    let terms = ListTermInfos::Termbase {
        termbase_id: &base_entry.id,
    }
    .run_on(&repo)
    .await
    .unwrap();

    let merged = terms.as_slice().first().unwrap();

    assert_eq!(merged.id, original.id);

    assert_eq!(merged.created_at, original.created_at);

    assert_eq!(merged.source, "Source");

    assert_eq!(merged.targets.as_slice().first().unwrap(), "Preferred");

    assert_eq!(merged.targets.len(), 3);

    assert!(
        merged.targets.contains(&"Second".into())
            && merged.targets.contains(&"Third".into())
    );

    verify_append_noop(&repo, &nucl, token, &base_entry.id).await;
}
