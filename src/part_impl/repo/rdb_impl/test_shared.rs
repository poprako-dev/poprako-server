use std::env;
use std::sync::OnceLock;

use diesel::prelude::*;
use diesel::{Connection, PgConnection};
use diesel_async::RunQueryDsl;
use diesel_migrations::{
    EmbeddedMigrations, MigrationHarness, embed_migrations,
};

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::chapter_model;
use crate::model::comic_model;
use crate::model::page_model;
use crate::model::team_model;
use crate::model::user_model;
use crate::model::workset_model;
use crate::part::repo::step::chapter::ChapterStep;
use crate::part::repo::step::comic::ComicStep;
use crate::part::repo::step::page::PageStep;
use crate::part::repo::step::team::TeamStep;
use crate::part::repo::step::user::UserStep;
use crate::part::repo::step::workset::WorksetStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, schema};
use crate::part_impl::shared::RdbCore;
use crate::part_impl::shared::result::diesel as diesel_error;
use crate::result::{RegularError, RegularResult};
use crate::util::DeriveTransactional as _;

pub struct UserFixture {
    pub user_form: user_model::Form,
}

pub struct TeamFixture {
    pub user_form: user_model::Form,
    pub team_form: team_model::Form,
}

pub struct WorksetFixture {
    pub team_form: team_model::Form,
    pub workset_form: workset_model::Form,
}

pub struct ComicFixture {
    pub creator_form: user_model::Form,
    pub team_form: team_model::Form,
    pub workset_form: workset_model::Form,
    pub comic_form: comic_model::Form,
}

pub struct ChapterFixture {
    pub creator_form: user_model::Form,
    pub team_form: team_model::Form,
    pub workset_form: workset_model::Form,
    pub comic_form: comic_model::Form,
    pub chapter_form: chapter_model::Form,
}

pub struct PageFixture {
    pub chapter_form: chapter_model::Form,
    pub page_form: page_model::Form,
}

const MIGRATIONS: EmbeddedMigrations = embed_migrations!("migrations");

static TEST_SCHEMA_READY: OnceLock<()> = OnceLock::new();

pub async fn shared() -> RdbCore {
    //
    dotenvy::dotenv().ok();

    let database_url = env::var("TEST_DATABASE_URL")
        .expect("TEST_DATABASE_URL must be set to run repo RDB tests");

    TEST_SCHEMA_READY.get_or_init(|| reset_test_schema(&database_url));

    RdbCore::from_database_url(&database_url).unwrap()
}

fn reset_test_schema(database_url: &str) {
    //
    let mut conn = PgConnection::establish(database_url).unwrap();

    MigrationHarness::revert_all_migrations(&mut conn, MIGRATIONS).unwrap();

    MigrationHarness::run_pending_migrations(&mut conn, MIGRATIONS).unwrap();
}

pub async fn reset(shared: &RdbCore, prefix: &str) {
    //
    cleanup(shared, prefix).await.unwrap();

    assert_no_leftovers(shared, prefix).await.unwrap();
}

pub async fn cleanup(shared: &RdbCore, prefix: &str) -> RegularResult<()> {
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

    diesel::delete(
        schema::t_chapter::table
            .filter(schema::t_chapter::f_id.like(&id_pattern)),
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

    Ok(())
}

pub async fn assert_no_leftovers(
    shared: &RdbCore,
    prefix: &str,
) -> RegularResult<()> {
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

    Ok(())
}

pub fn user_form(prefix: &str, name: &str) -> user_model::Form {
    user_model::Form {
        id: format!("{}user-{}", prefix, name),
        nickname: format!("RDB User {}", name),
        qid: format!("{}qid-{}", prefix, name),
        password_hash: "hash".into(),
    }
}

pub fn team_form(prefix: &str) -> team_model::Form {
    team_model::Form {
        id: format!("{}team", prefix),
        name: "RDB Team".into(),
        description: "team".into(),
    }
}

pub fn workset_form(
    prefix: &str,
    team_form: &team_model::Form,
) -> workset_model::Form {
    workset_model::Form {
        id: format!("{}workset", prefix),
        team_id: team_form.id.clone(),
        index: 0,
        name: "RDB Workset".into(),
        description: Some("workset".into()),
    }
}

pub fn comic_form(
    prefix: &str,
    workset_form: &workset_model::Form,
    creator_form: &user_model::Form,
) -> comic_model::Form {
    comic_model::Form {
        id: format!("{}comic", prefix),
        workset_id: workset_form.id.clone(),
        index: 0,
        title: "RDB Comic".into(),
        author: "RDB Author".into(),
        description: Some("comic".into()),
        creator_id: creator_form.id.clone(),
    }
}

pub fn chapter_form(
    prefix: &str,
    comic_form: &comic_model::Form,
    creator_form: &user_model::Form,
) -> chapter_model::Form {
    chapter_model::Form {
        id: format!("{}chapter", prefix),
        comic_id: comic_form.id.clone(),
        is_pinned: true,
        index: 0,
        subtitle: "RDB Chapter".into(),
        creator_id: creator_form.id.clone(),
    }
}

pub fn page_form(
    prefix: &str,
    chapter_form: &chapter_model::Form,
) -> page_model::Form {
    page_model::Form {
        id: format!("{}page", prefix),
        chapter_id: chapter_form.id.clone(),
        index: 0,
        image_key: None,
        image_version: 0,
    }
}

pub async fn create_user(shared: &RdbCore, user_form: &user_model::Form) {
    //
    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &UserStep::create(user_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();
}

pub async fn seed_user(shared: &RdbCore, prefix: &str) -> UserFixture {
    //
    reset(shared, prefix).await;

    let user_form = user_form(prefix, "owner");

    create_user(shared, &user_form).await;

    UserFixture { user_form }
}

pub async fn seed_user_and_team(shared: &RdbCore, prefix: &str) -> TeamFixture {
    //
    reset(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone());

    let user_form = user_form(prefix, "owner");

    let team_form = team_form(prefix);

    create_user(shared, &user_form).await;

    Execute::execute(&repo, &TeamStep::create(&team_form))
        .await
        .ok()
        .unwrap();

    TeamFixture {
        user_form,
        team_form,
    }
}

pub async fn seed_workset(shared: &RdbCore, prefix: &str) -> WorksetFixture {
    //
    let team_fixture = seed_user_and_team(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let workset_form = workset_form(prefix, &team_fixture.team_form);

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &WorksetStep::create(&workset_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    WorksetFixture {
        team_form: team_fixture.team_form,
        workset_form,
    }
}

pub async fn seed_comic(shared: &RdbCore, prefix: &str) -> ComicFixture {
    //
    reset(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let creator_form = user_form(prefix, "creator");

    let team_form = team_form(prefix);

    let workset_form = workset_form(prefix, &team_form);

    let comic_form = comic_form(prefix, &workset_form, &creator_form);

    create_user(shared, &creator_form).await;

    Execute::execute(&repo, &TeamStep::create(&team_form))
        .await
        .ok()
        .unwrap();

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &WorksetStep::create(&workset_form),
            )
            .await?;

            Advance::advance(
                &transactional_repo,
                context,
                &ComicStep::create(&comic_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    ComicFixture {
        creator_form,
        team_form,
        workset_form,
        comic_form,
    }
}

pub async fn seed_chapter(shared: &RdbCore, prefix: &str) -> ChapterFixture {
    //
    let comic_fixture = seed_comic(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let chapter_form = chapter_form(
        prefix,
        &comic_fixture.comic_form,
        &comic_fixture.creator_form,
    );

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &ChapterStep::create(&chapter_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    ChapterFixture {
        creator_form: comic_fixture.creator_form,
        team_form: comic_fixture.team_form,
        workset_form: comic_fixture.workset_form,
        comic_form: comic_fixture.comic_form,
        chapter_form,
    }
}

pub async fn seed_page(shared: &RdbCore, prefix: &str) -> PageFixture {
    //
    let chapter_fixture = seed_chapter(shared, prefix).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let page_form = page_form(prefix, &chapter_fixture.chapter_form);

    drive
        .with_context(async |context| {
            //
            Advance::advance(
                &transactional_repo,
                context,
                &PageStep::create_batch(std::slice::from_ref(&page_form)),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    PageFixture {
        chapter_form: chapter_fixture.chapter_form,
        page_form,
    }
}
