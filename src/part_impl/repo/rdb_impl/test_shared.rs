mod cleanup;

pub mod form;

use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use poprako_rdb_core::RdbCore;

use crate::model::write::chapter::ChapterEntry;
use crate::model::write::comic::ComicEntry;
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
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared::form::{
    chapter_entry, comic_entry, page_entry, team_entry, user_entry,
    workset_entry,
};
use crate::result::BaseError;

pub use cleanup::{assert_no_leftovers, cleanup};

pub struct UserFixture {
    pub user_entry: UserEntry,
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
    pub comic_entry: ComicEntry,
}

pub struct ChapterFixture {
    //
    pub creator_form: UserEntry,

    pub team_entry: TeamEntry,

    pub workset_entry: WorksetEntry,

    pub comic_entry: ComicEntry,

    pub chapter_entry: ChapterEntry,
}

pub struct PageFixture {
    //
    pub team_entry: TeamEntry,
    pub chapter_entry: ChapterEntry,
    pub page_entry: PageEntry,
}

pub async fn reset(shared: &RdbCore, prefix: &str) {
    //
    cleanup(shared, prefix).await.unwrap();

    assert_no_leftovers(shared, prefix).await.unwrap();
}

pub async fn create_user(shared: &RdbCore, user_entry: &UserEntry) {
    //
    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    nucl.coord(async |context| {
        //
        repo.step(context, &CreateUser { entry: user_entry })
            .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();
}

pub async fn seed_user(shared: &RdbCore, prefix: &str) -> UserFixture {
    //
    reset(shared, prefix).await;

    let user_entry = user_entry(prefix, "owner");

    create_user(shared, &user_entry).await;

    UserFixture { user_entry }
}

pub async fn seed_user_and_team(shared: &RdbCore, prefix: &str) -> TeamFixture {
    //
    reset(shared, prefix).await;

    let repo = HybRepo::new(shared.clone());

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

pub async fn seed_workset(shared: &RdbCore, prefix: &str) -> WorksetFixture {
    //
    let team_fixture = seed_user_and_team(shared, prefix).await;

    let repo = HybRepo::new(shared.clone());

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

pub async fn seed_comic(shared: &RdbCore, prefix: &str) -> ComicFixture {
    //
    reset(shared, prefix).await;

    let repo = HybRepo::new(shared.clone());

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

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateWorkset {
                entry: &workset_entry,
            },
        )
        .await?;

        repo.step(
            context,
            &CreateComic {
                entry: &comic_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    ComicFixture {
        creator_form,
        team_entry,
        workset_entry,
        comic_entry,
    }
}

pub async fn seed_chapter(shared: &RdbCore, prefix: &str) -> ChapterFixture {
    //
    let comic_fixture = seed_comic(shared, prefix).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let chapter_entry = chapter_entry(
        prefix,
        &comic_fixture.comic_entry,
        &comic_fixture.creator_form,
    );

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateChapter {
                entry: &chapter_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    ChapterFixture {
        creator_form: comic_fixture.creator_form,
        team_entry: comic_fixture.team_entry,
        workset_entry: comic_fixture.workset_entry,
        comic_entry: comic_fixture.comic_entry,
        chapter_entry,
    }
}

pub async fn seed_page(shared: &RdbCore, prefix: &str) -> PageFixture {
    //
    let chapter_fixture = seed_chapter(shared, prefix).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<ReptRead>::new(shared.clone());

    let page_entry = page_entry(prefix, &chapter_fixture.chapter_entry);

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
        chapter_entry: chapter_fixture.chapter_entry,
        page_entry,
    }
}
