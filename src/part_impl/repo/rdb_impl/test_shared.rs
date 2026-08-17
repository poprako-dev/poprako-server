mod form;

use diesel::prelude::*;
use diesel_async::RunQueryDsl;
use poprako_orchestra::{Nucl as _, Run as _, Step as _};

use crate::model::write::chapter::ChapterEntry;
use crate::model::write::comic::ComicEntry;
use crate::model::write::page::PageEntry;
use crate::model::write::team::TeamEntry;
use crate::model::write::user::UserEntry;
use crate::model::write::workset::WorksetEntry;
use crate::part::repo::oper::chapter::CreateChapter;
use crate::part::repo::oper::comic::CreateComic;
use crate::part::repo::oper::page::CreatePages;
use crate::part::repo::oper::team::CreateTeam;
use crate::part::repo::oper::user::CreateUser;
use crate::part::repo::oper::workset::CreateWorkset;
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::schema;
use crate::result::{BaseError, BaseRest, accept};
use crate::shared::RdbCore;
use crate::shared::result::diesel as diesel_error;

pub use self::form::{
    chapter_entry, comic_entry, page_entry, team_entry, user_entry,
    workset_entry,
};

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

pub async fn cleanup(shared: &RdbCore, prefix: &str) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    diesel::delete(
        schema::t_comment::table
            .filter(schema::t_comment::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_announcement::table
            .filter(schema::t_announcement::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_assignment_invitation::table
            .filter(schema::t_assignment_invitation::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_assignment::table
            .filter(schema::t_assignment::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_unit::table.filter(schema::t_unit::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_page::table.filter(schema::t_page::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(schema::t_chapter_workflow_record::table.filter(
        schema::t_chapter_workflow_record::f_chapter_id.like(&id_pattern),
    ))
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_chapter::table
            .filter(schema::t_chapter::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_term::table.filter(schema::t_term::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_termbase::table
            .filter(schema::t_termbase::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_comic::table.filter(schema::t_comic::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_workset::table
            .filter(schema::t_workset::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_member_invitation::table
            .filter(schema::t_member_invitation::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_member::table
            .filter(schema::t_member::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_system_mail::table
            .filter(schema::t_system_mail::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_local_message::table
            .filter(schema::t_local_message::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_team::table.filter(schema::t_team::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    diesel::delete(
        schema::t_user::table.filter(schema::t_user::f_id.like(&id_pattern)),
    )
    .execute(&mut conn)
    .await
    .map_err(diesel_error)?;

    accept(())
}

pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> BaseRest<()> {
    //
    let mut conn = shared.get().await?;

    let id_pattern = format!("{}%", prefix);

    let announcement_count: i64 = schema::t_announcement::table
        .filter(schema::t_announcement::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let assignment_count: i64 = schema::t_assignment::table
        .filter(schema::t_assignment::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let assignment_invitation_count: i64 =
        schema::t_assignment_invitation::table
            .filter(schema::t_assignment_invitation::f_id.like(&id_pattern))
            .count()
            .get_result(&mut conn)
            .await
            .map_err(diesel_error)?;

    let chapter_count: i64 = schema::t_chapter::table
        .filter(schema::t_chapter::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let chapter_workflow_record_count: i64 =
        schema::t_chapter_workflow_record::table
            .filter(
                schema::t_chapter_workflow_record::f_chapter_id
                    .like(&id_pattern),
            )
            .count()
            .get_result(&mut conn)
            .await
            .map_err(diesel_error)?;

    let comic_count: i64 = schema::t_comic::table
        .filter(schema::t_comic::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let comment_count: i64 = schema::t_comment::table
        .filter(schema::t_comment::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let local_message_count: i64 = schema::t_local_message::table
        .filter(schema::t_local_message::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let member_count: i64 = schema::t_member::table
        .filter(schema::t_member::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let member_invitation_count: i64 = schema::t_member_invitation::table
        .filter(schema::t_member_invitation::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let page_count: i64 = schema::t_page::table
        .filter(schema::t_page::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let system_mail_count: i64 = schema::t_system_mail::table
        .filter(schema::t_system_mail::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let team_count: i64 = schema::t_team::table
        .filter(schema::t_team::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let unit_count: i64 = schema::t_unit::table
        .filter(schema::t_unit::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let user_count: i64 = schema::t_user::table
        .filter(schema::t_user::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    let workset_count: i64 = schema::t_workset::table
        .filter(schema::t_workset::f_id.like(&id_pattern))
        .count()
        .get_result(&mut conn)
        .await
        .map_err(diesel_error)?;

    assert_eq!(announcement_count, 0);

    assert_eq!(assignment_count, 0);

    assert_eq!(assignment_invitation_count, 0);

    assert_eq!(chapter_count, 0);

    assert_eq!(chapter_workflow_record_count, 0);

    assert_eq!(comic_count, 0);

    assert_eq!(comment_count, 0);

    assert_eq!(local_message_count, 0);

    assert_eq!(member_count, 0);

    assert_eq!(member_invitation_count, 0);

    assert_eq!(page_count, 0);

    assert_eq!(system_mail_count, 0);

    assert_eq!(team_count, 0);

    assert_eq!(unit_count, 0);

    assert_eq!(user_count, 0);

    assert_eq!(workset_count, 0);

    accept(())
}

pub async fn create_user(shared: &RdbCore, user_entry: &UserEntry) {
    //
    let repo = HybRepo::new(shared.clone());

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

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

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

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

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

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

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

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

    let nucl =
        RdbNucl::<crate::part::nucl::RepeatableRead>::new(shared.clone());

    let page_entry = page_entry(prefix, &chapter_fixture.chapter_entry);

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreatePages {
                entries: std::slice::from_ref(&page_entry),
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
