#![allow(
    clippy::unwrap_used,
    reason = "Test fixtures and assertions fail immediately when their invariants are violated"
)]

mod cleanup;

pub mod form;

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::config::repo::RepoConfig;
use crate::model::read::proj::chapter::ChapterInfo;
use crate::model::read::proj::comic::ComicInfo;
use crate::model::write::page::{PageEntry, PageManifestEntry};
use crate::model::write::team::TeamEntry;
use crate::model::write::user::UserEntry;
use crate::model::write::workset::WorksetEntry;
use crate::part::nucl::ReptRead;
use crate::part::repo::oper::chapter::CreateChapter;
use crate::part::repo::oper::comic::CreateComic;
use crate::part::repo::oper::page::ApplyPageManifest;
use crate::part::repo::oper::team::CreateTeam;
use crate::part::repo::oper::user::CreateUser;
use crate::part::repo::oper::workset::CreateWorkset;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::RdbRepo;
use crate::part_impl::repo::rdb_impl::test_shared::form::{
    chapter_entry, comic_entry, page_entry, team_entry, user_entry,
    workset_entry,
};
use crate::result::BaseError;
use poprako_rdb_core::RdbCore;

pub use cleanup::{assert_no_leftovers, cleanup};

pub struct UserFixture {
    pub entry: UserEntry,
}

pub struct TeamFixture {
    //
    pub user_entry: UserEntry,
    pub team_entry: TeamEntry,
}

pub struct WorksetFixture {
    //
    pub team_entry: TeamEntry,
    pub workset_entry: WorksetEntry,
}

pub struct ComicFixture {
    //
    pub creator_form: UserEntry,
    pub team_entry: TeamEntry,
    pub workset_entry: WorksetEntry,
    pub comic_info: ComicInfo,
}

pub struct ChapterFixture {
    //
    pub creator_form: UserEntry,

    pub team_entry: TeamEntry,

    pub workset_entry: WorksetEntry,

    pub comic_info: ComicInfo,

    pub chapter_info: ChapterInfo,
}

pub struct PageFixture {
    //
    pub team_entry: TeamEntry,
    pub chapter_info: ChapterInfo,
    pub page_entry: PageEntry,
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn reset(shared: &RdbCore, prefix: &str) {
    //
    cleanup(shared, prefix).await.unwrap();

    assert_no_leftovers(shared, prefix).await.unwrap();
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn create_user(shared: &RdbCore, entry: &UserEntry) {
    //
    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        //
        repo.step(context, &CreateUser { entry }).await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();
}

pub async fn seed_user(shared: &RdbCore, prefix: &str) -> UserFixture {
    //
    reset(shared, prefix).await;

    let entry = user_entry(prefix, "owner");

    create_user(shared, &entry).await;

    UserFixture { entry }
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn seed_user_and_team(shared: &RdbCore, prefix: &str) -> TeamFixture {
    //
    reset(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let user_entry = user_entry(prefix, "owner");

    let team_entry = team_entry(prefix);

    create_user(shared, &user_entry).await;

    repo.run(&CreateTeam { entry: &team_entry })
        .await
        .ok()
        .unwrap();

    TeamFixture {
        user_entry,
        team_entry,
    }
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn seed_workset(shared: &RdbCore, prefix: &str) -> WorksetFixture {
    //
    let team_fixture = seed_user_and_team(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let workset_entry = workset_entry(prefix, &team_fixture.team_entry);

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateWorkset {
                entry: &workset_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    WorksetFixture {
        team_entry: team_fixture.team_entry,
        workset_entry,
    }
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn seed_comic(shared: &RdbCore, prefix: &str) -> ComicFixture {
    //
    reset(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let creator_form = user_entry(prefix, "creator");

    let team_entry = team_entry(prefix);

    let workset_entry = workset_entry(prefix, &team_entry);

    let comic_entry = comic_entry(prefix, &workset_entry, &creator_form);

    create_user(shared, &creator_form).await;

    repo.run(&CreateTeam { entry: &team_entry })
        .await
        .ok()
        .unwrap();

    let comic_info = nucl
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreateWorkset {
                    entry: &workset_entry,
                },
            )
            .await?;

            let comic_info = repo
                .step(
                    context,
                    &CreateComic {
                        entry: &comic_entry,
                    },
                )
                .await?;

            Ok::<_, BaseError>(comic_info)
        })
        .await
        .ok()
        .unwrap();

    ComicFixture {
        creator_form,
        team_entry,
        workset_entry,
        comic_info,
    }
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn seed_chapter(shared: &RdbCore, prefix: &str) -> ChapterFixture {
    //
    let comic_fixture = seed_comic(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let entry = chapter_entry(
        prefix,
        &comic_fixture.comic_info,
        &comic_fixture.creator_form,
    );

    let chapter_info = nucl
        .coord(async |context| {
            //
            let chapter_info =
                repo.step(context, &CreateChapter { entry: &entry }).await?;

            Ok::<_, BaseError>(chapter_info)
        })
        .await
        .ok()
        .unwrap();

    ChapterFixture {
        creator_form: comic_fixture.creator_form,
        team_entry: comic_fixture.team_entry,
        workset_entry: comic_fixture.workset_entry,
        comic_info: comic_fixture.comic_info,
        chapter_info,
    }
}

/// # Panics
/// Panics if fixture setup fails or a scenario assertion is violated.
pub async fn seed_page(shared: &RdbCore, prefix: &str) -> PageFixture {
    //
    let chapter_fixture = seed_chapter(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone(), RepoConfig::default());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_entry = page_entry(prefix, &chapter_fixture.chapter_info);

    let page_manifest_entry = PageManifestEntry {
        id: page_entry.id.clone(),
        chapter_id: page_entry.chapter_id.clone(),
        index: page_entry.index,
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &ApplyPageManifest {
                entries: std::slice::from_ref(&page_manifest_entry),
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    PageFixture {
        team_entry: chapter_fixture.team_entry,
        chapter_info: chapter_fixture.chapter_info,
        page_entry,
    }
}
