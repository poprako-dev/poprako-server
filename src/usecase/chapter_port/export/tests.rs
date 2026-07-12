// export(export)(positive): assignee exports chapter metadata, signed uploaded page URLs, and ordered units.
// export_label_plus(export_label_plus)(positive): assignee exports ordered pages and units as LabelPlus text.

use super::*;

use time::OffsetDateTime;

use crate::model::assignment_model;
use crate::model::chapter_model;
use crate::model::comic_model;
use crate::model::page_model;
use crate::model::unit_model;
use crate::model::user_model;
use crate::model::workset_model;
use crate::part_impl::repo::mock_impl::Mock;
use crate::value::chapter::StageMask;
use crate::value::role::{RoleField, RoleMask};

fn token(user_id: &str) -> user_model::Token {
    user_model::Token {
        user_id: user_id.into(),
    }
}

fn comic(id: &str) -> comic_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    comic_model::Info {
        id: id.into(),
        workset_id: "workset-1".into(),
        index: 0,
        title: "Pop Comic".into(),
        author: "author".into(),
        description: None,
        cover_key: None,
        cover_uploaded: false,
        cover_version: 0,
        chapter_count: 1,
        chapter_next_index: 1,
        creator_id: "user-1".into(),
        workset: None,
        team: None,
        creator: None,
        last_active_at: time,
        created_at: time,
        updated_at: time,
    }
}

fn workset(id: &str) -> workset_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    workset_model::Info {
        id: id.into(),
        team_id: "team-1".into(),
        index: 0,
        name: "workset".into(),
        description: None,
        comic_count: 1,
        comic_next_index: 1,
        created_at: time,
        updated_at: time,
    }
}

fn chapter(id: &str) -> chapter_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    chapter_model::Info {
        id: id.into(),
        comic_id: "comic-1".into(),
        is_pinned: true,
        index: 3,
        subtitle: "Arrival".into(),
        page_count: 2,
        total_unit_count: 2,
        translated_unit_count: 2,
        proofread_unit_count: 1,
        stages: StageMask::try_from(0u32).ok().unwrap(),
        creator_id: "user-1".into(),
        comic: None,
        creator: None,
        created_at: time,
        updated_at: time,
    }
}

fn assignment(
    chapter_id: &str,
    user_id: &str,
    role_mask: RoleMask,
) -> assignment_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    assignment_model::Info {
        id: format!("assignment-{}-{}", chapter_id, user_id),
        chapter_id: chapter_id.into(),
        user_id: user_id.into(),
        user: None,
        chapter: None,
        roles: role_mask,
        created_at: time,
        updated_at: time,
    }
}

fn page(
    id: &str,
    index: i32,
    image_key: Option<&str>,
    image_uploaded: bool,
) -> page_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    page_model::Info {
        id: id.into(),
        chapter_id: "chapter-1".into(),
        index,
        image_key: image_key.map(Into::into),
        image_uploaded,
        image_version: 1,
        total_unit_count: 1,
        translated_unit_count: 1,
        proofread_unit_count: 0,
        created_at: time,
        updated_at: time,
    }
}

fn unit(
    id: &str,
    page_id: &str,
    index: i32,
    text: &str,
    proofread_text: Option<&str>,
) -> unit_model::Info {
    //
    let time = OffsetDateTime::now_utc();

    unit_model::Info {
        id: id.into(),
        page_id: page_id.into(),
        index,
        is_bubble: true,
        is_proofread: proofread_text.is_some(),
        x_coord: 0.25,
        y_coord: 0.75,
        translated_text: Some(text.into()),
        last_translator_id: Some("translator-1".into()),
        proofread_text: proofread_text.map(Into::into),
        last_proofreader_id: Some("proofreader-1".into()),
        created_at: time,
        updated_at: time,
    }
}

fn seed_scope(mock: &Mock) {
    //
    mock.seed_workset(workset("workset-1"));

    mock.seed_comic(comic("comic-1"));

    mock.seed_chapter(chapter("chapter-1"));

    mock.seed_page(page("page-1", 0, Some("one.png"), true));

    mock.seed_page(page("page-2", 1, Some("two.png"), false));

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::PROOFREADER),
    ));
}

#[tokio::test]
async fn export_returns_chapter_pages_and_units() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_unit(unit("unit-b", "page-1", 1, "beta", None));

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", Some("alpha proof")));

    let exported =
        export(&mock, &mock, token("user-1"), "chapter-1".into()).await;

    let exported = match exported {
        //
        Ok(exported) => exported,

        Err(_) => panic!("expected export success"),
    };

    assert_eq!(exported.chapter_id, "chapter-1");

    assert_eq!(exported.chapter_index, 3);

    assert_eq!(exported.chapter_subtitle, Some("Arrival".into()));

    assert_eq!(exported.comic_title, "Pop Comic");

    assert_eq!(exported.pages.len(), 2);

    assert_eq!(
        exported.pages[0].image_url,
        Some("https://test.local/get/one.png".into())
    );

    assert_eq!(exported.pages[1].image_url, None);

    assert_eq!(exported.pages[0].units.len(), 2);

    assert_eq!(exported.pages[0].units[0].unit_id, "unit-a");

    assert_eq!(
        exported.pages[0].units[0].proofread_text,
        Some("alpha proof".into())
    );
}

#[tokio::test]
async fn export_label_plus_returns_text_payload() {
    //
    let mock = Mock::new();

    seed_scope(&mock);

    mock.seed_unit(unit("unit-a", "page-1", 0, "alpha", Some("alpha proof")));

    let exported =
        export_label_plus(&mock, token("user-1"), "chapter-1".into()).await;

    let exported = match exported {
        //
        Ok(exported) => exported,

        Err(_) => panic!("expected LabelPlus export success"),
    };

    assert!(exported.contains("Exported by PopRaKo Web"));

    assert!(exported.contains(">>>>>>>>[000.png]<<<<<<<<"));

    assert!(
        exported
            .contains("----------------[1]----------------[0.2500,0.7500,1]")
    );

    assert!(exported.contains("alpha proof"));
}
