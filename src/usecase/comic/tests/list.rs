// list_infos(list_infos)(positive): fuzzy title should narrow results by display index, title, or author substring.
// list_infos(list_infos)(positive): pinned chapter assignments should be returned in comic order.
// list_infos(list_infos)(negative): pinned chapter assignments without pinned chapters should return an argument error.

use super::*;

#[tokio::test]
async fn list_infos_returns_pinned_chapter_assignments_in_comic_order() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(comic("comic-2", "workset-1", 2));

    mock.seed_comic(comic("comic-1", "workset-1", 1));

    mock.seed_chapter(chapter(
        "chapter-1",
        "comic-1",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_chapter(chapter(
        "chapter-2",
        "comic-2",
        StageMask::try_from(0u32).ok().unwrap(),
    ));

    mock.seed_assignment(assignment("assignment-1", "chapter-1", "user-1"));

    mock.seed_assignment(assignment("assignment-2", "chapter-1", "user-2"));

    mock.seed_assignment(assignment("assignment-3", "chapter-2", "user-3"));

    let list = list_infos((&mock, &mock,),
        token("user-1"),
        ListComicInfosParams {
            incl_opt: Vec::new(),
            with_opt: vec![
                ComicWithOpt::PinnedChapter,
                ComicWithOpt::PinnedChapterAssignment,
            ],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(list.pinned_chapter_assignments.len(), list.comics.len());

    assert_eq!(list.pinned_chapter_assignments[0].len(), 2);

    assert_eq!(
        list.pinned_chapter_assignments[0][0].chapter_id,
        "chapter-1"
    );

    assert_eq!(
        list.pinned_chapter_assignments[0][1].chapter_id,
        "chapter-1"
    );

    assert_eq!(list.pinned_chapter_assignments[1].len(), 1);

    assert_eq!(
        list.pinned_chapter_assignments[1][0].chapter_id,
        "chapter-2"
    );
}

#[tokio::test]
async fn list_infos_rejects_assignments_without_pinned_chapters() {
    //
    let mock = Mock::new();

    let err = list_infos((&mock, &mock,),
        token("user-1"),
        ListComicInfosParams {
            incl_opt: Vec::new(),
            with_opt: vec![ComicWithOpt::PinnedChapterAssignment],
            workset_id: "workset-1".into(),
            fuzzy_title: None,
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
}

#[tokio::test]
async fn list_infos_filters_by_fuzzy_title() {
    //
    let mock = Mock::new();

    mock.seed_workset(workset("workset-1", "team-1"));

    mock.seed_member(admin_member("user-1", "team-1"));

    mock.seed_comic(ComicInfo {
        title: "Alpha Adventure".into(),
        author: "Alice".into(),
        ..comic("comic-alpha", "workset-1", 0)
    });

    mock.seed_comic(ComicInfo {
        title: "Beta Journey".into(),
        author: "Bob".into(),
        ..comic("comic-beta", "workset-1", 1)
    });

    mock.seed_comic(ComicInfo {
        title: "Gamma Quest".into(),
        author: "Carol".into(),
        ..comic("comic-gamma", "workset-1", 2)
    });

    let list = list_infos((&mock, &mock,),
        token("user-1"),
        ListComicInfosParams {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("Beta".into()),
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics[0].id, "comic-beta");

    let list = list_infos((&mock, &mock,),
        token("user-1"),
        ListComicInfosParams {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("Carol".into()),
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics[0].id, "comic-gamma");

    let list = list_infos((&mock, &mock,),
        token("user-1"),
        ListComicInfosParams {
            incl_opt: Vec::new(),
            with_opt: vec![],
            workset_id: "workset-1".into(),
            fuzzy_title: Some("1".into()),
            stages: None,
            offset: 0,
            limit: 10,
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(list.comics.len(), 1);

    assert_eq!(list.comics[0].id, "comic-alpha");
}
