// list_infos(list_infos)(positive): fuzzy title should narrow results by display index, title, or author substring.
// list_infos(list_infos)(positive): pinned chapter assignments with users should be returned in comic order.
// list_infos(list_infos)(negative): pinned chapter assignments without pinned chapters should return an argument error.

use super::*;

use crate::test_util::now;
use crate::value::comic::ComicStatus;

// Seed distinct assignees on two pinned chapters for inclusion checks.
fn pinned_assignment_scope() -> Mock {
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    mock.seed_user(
        user("user--00000000001", "user--00000000001", "User One"),
        invalid_credential("user--00000000001"),
    );

    mock.seed_user(
        user("user--00000000002", "user--00000000002", "User Two"),
        invalid_credential("user--00000000002"),
    );

    mock.seed_user(
        user("user--00000000003", "user--00000000003", "User Three"),
        invalid_credential("user--00000000003"),
    );

    mock.seed_comic(comic("comic--00000000002", "workset--00000000001", 2));

    mock.seed_comic(comic("comic--00000000001", "workset--00000000001", 1));

    mock.seed_chapter(chapter(
        "chapter--00000000001",
        "comic--00000000001",
        StageMask::try_from(0u32).unwrap(),
    ));

    mock.seed_chapter(chapter(
        "chapter--00000000002",
        "comic--00000000002",
        StageMask::try_from(0u32).unwrap(),
    ));

    mock.seed_assignment(assignment(
        "assignment--00000000001",
        "chapter--00000000001",
        "user--00000000001",
    ));

    mock.seed_assignment(assignment(
        "assignment--00000000002",
        "chapter--00000000001",
        "user--00000000002",
    ));

    mock.seed_assignment(assignment(
        "assignment--00000000003",
        "chapter--00000000002",
        "user--00000000003",
    ));

    mock
}

#[tokio::test]
async fn list_infos_filters_by_lifecycle_status() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    mock.seed_comic(comic("comic--28999254184", "workset--00000000001", 0));

    let mut archived_comic_info =
        comic("comic--40413916896", "workset--00000000001", 1);

    archived_comic_info.archived_at = Some(now());

    mock.seed_comic(archived_comic_info);

    let active_list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: Vec::new(),
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: Some(ComicStatus::Active),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(active_list.comics.len(), 1);

    assert_eq!(active_list.comics.first().unwrap().id, "comic--28999254184");

    let archived_list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: Vec::new(),
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: Some(ComicStatus::Archived),
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(archived_list.comics.len(), 1);

    assert_eq!(
        archived_list.comics.first().unwrap().id,
        "comic--40413916896"
    );
}

#[tokio::test]
async fn list_infos_includes_users_in_pinned_chapter_assignments() {
    //
    let mock = pinned_assignment_scope();

    let list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![
                ComicWithOpt::PinnedChapter,
                ComicWithOpt::PinnedChapterAssignment,
            ],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.pinned_chapter_assignments.len(), list.comics.len());

    assert_eq!(list.pinned_chapter_assignments.first().unwrap().len(), 2);

    assert_eq!(
        list.pinned_chapter_assignments
            .first()
            .unwrap()
            .first()
            .unwrap()
            .chapter_id,
        "chapter--00000000001"
    );

    assert_eq!(
        list.pinned_chapter_assignments
            .first()
            .unwrap()
            .iter()
            .find(|assignment_view| assignment_view.user_id
                == "user--00000000001")
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .nickname,
        "User One"
    );

    assert_eq!(
        list.pinned_chapter_assignments
            .first()
            .unwrap()
            .get(1)
            .unwrap()
            .chapter_id,
        "chapter--00000000001"
    );

    assert_eq!(
        list.pinned_chapter_assignments
            .first()
            .unwrap()
            .iter()
            .find(|assignment_view| assignment_view.user_id
                == "user--00000000002")
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .nickname,
        "User Two"
    );

    assert_eq!(list.pinned_chapter_assignments.get(1).unwrap().len(), 1);

    assert_eq!(
        list.pinned_chapter_assignments
            .get(1)
            .unwrap()
            .first()
            .unwrap()
            .chapter_id,
        "chapter--00000000002"
    );

    assert_eq!(
        list.pinned_chapter_assignments
            .get(1)
            .unwrap()
            .first()
            .unwrap()
            .user
            .as_ref()
            .unwrap()
            .nickname,
        "User Three"
    );
}

#[tokio::test]
async fn list_infos_rejects_assignments_without_pinned_chapters() {
    //
    let mock = Mock::new();

    let error = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![ComicWithOpt::PinnedChapterAssignment],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: None,
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(&error, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_by_fuzzy_title() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset--00000000001", "team--00000000001"));

    mock.seed_member(admin_member("user--00000000001", "team--00000000001"));

    mock.seed_comic(ComicInfo {
        title: "Alpha Adventure".into(),
        author: "Alice".into(),
        ..comic("comic--98112322974", "workset--00000000001", 0)
    });

    mock.seed_comic(ComicInfo {
        title: "Beta Journey".into(),
        author: "Bob".into(),
        ..comic("comic--38335660265", "workset--00000000001", 1)
    });

    mock.seed_comic(ComicInfo {
        title: "Gamma Quest".into(),
        author: "Carol".into(),
        ..comic("comic--36464404672", "workset--00000000001", 2)
    });

    let list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: Some("\u{2003}Beta\u{00a0}".into()),
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics.first().unwrap().id, "comic--38335660265");

    let list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: Some("Carol".into()),
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics.first().unwrap().id, "comic--36464404672");

    let list = list_infos(
        (&mock, &mock),
        token("user--00000000001"),
        ListComicInfosInstr {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset--00000000001".into(),
            fuzzy_title: Some("1".into()),
            stages: None,
            status: None,
            offset: 0,
            limit: crate::value::pagination::PubListLimit::new(10).unwrap(),
        },
    )
    .await
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics.first().unwrap().id, "comic--98112322974");
}
