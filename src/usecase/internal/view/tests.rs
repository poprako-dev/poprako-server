//! Complete response presentation contracts at the internal rendering interface.

mod fixture;
mod pinned_chapter;

use crate::usecase::internal::view::{
    assignment_info_view, assignment_info_views, chapter_info_views,
    comic_info_view,
};

use fixture::{TestObjDept, TestRepo, assignment_info};

// comic_uses_pinned_first_page_when_dedicated_cover_is_absent(comic_info_view)(positive): a missing dedicated cover uses the pinned chapter's first page.
// comic_prefers_dedicated_cover_over_pinned_first_page(comic_info_view)(positive): available dedicated covers win over page images.
// chapter_list_preserves_order_and_renders_nested_fallback(chapter_info_views)(positive): chapter order and nested comic fallback survive complete rendering.
// nested_repeated_models_load_once_per_object_marker(assignment_info_views)(positive): repeated nested models share one deduplicated batch per object marker.
// empty_lists_perform_no_object_or_repository_operations(chapter_info_views)(positive): empty rendering avoids all object and fallback reads.
// partial_assignment_skips_absent_markers(assignment_info_view)(positive): absent included models require no unrelated object work.
// assignment_list_deduplicates_and_sorts_each_batch(assignment_info_views)(positive): object lookup order is deterministic without changing response order.
// metadata_error_is_propagated_without_url_generation(assignment_info_view)(negative): metadata failure prevents URL generation.
// url_error_is_propagated_after_metadata_load(assignment_info_view)(negative): URL failure is not converted to an incomplete success.
// single_assignment_does_not_load_cover_fallback(assignment_info_view)(positive): single-assignment rendering needs no repository and retains missing cover URLs.

#[tokio::test]
async fn comic_uses_pinned_first_page_when_dedicated_cover_is_absent() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    obj_dept.omit("cover-list", "comic-1");

    let comic_info = assignment_info().chapter.unwrap().comic.unwrap();

    let comic_view =
        comic_info_view(&repo, &obj_dept, comic_info).await.unwrap();

    assert_eq!(
        comic_view.cover_url.as_deref(),
        Some("https://obj.test/page-1")
    );

    assert_eq!(
        comic_view.cover_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/page-1")
    );
}

#[tokio::test]
async fn comic_prefers_dedicated_cover_over_pinned_first_page() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    let comic_info = assignment_info().chapter.unwrap().comic.unwrap();

    let comic_view =
        comic_info_view(&repo, &obj_dept, comic_info).await.unwrap();

    assert_eq!(
        comic_view.cover_url.as_deref(),
        Some("https://obj.test/comic-1")
    );

    assert_eq!(
        comic_view.cover_thumbnail_url.as_deref(),
        Some("https://obj.test/thumbnail/comic-1")
    );
}

#[tokio::test]
async fn chapter_list_preserves_order_and_renders_nested_fallback() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    obj_dept.omit("cover-list", "comic-1");

    let first_chapter = assignment_info().chapter.unwrap();

    let mut second_chapter = first_chapter.clone();

    second_chapter.id = "chapter-2".into();

    let chapter_views = chapter_info_views(
        &repo,
        &obj_dept,
        vec![second_chapter, first_chapter],
    )
    .await
    .unwrap();

    assert_eq!(
        chapter_views
            .iter()
            .map(|view| view.id.as_str())
            .collect::<Vec<_>>(),
        vec!["chapter-2", "chapter-1"]
    );

    for chapter_view in chapter_views {
        //
        assert_eq!(
            chapter_view.comic.unwrap().cover_url.as_deref(),
            Some("https://obj.test/page-1")
        );

        assert_eq!(
            chapter_view.creator.unwrap().avatar_url.as_deref(),
            Some("https://obj.test/user-1")
        );
    }

    assert_eq!(
        obj_dept.calls("cover-list"),
        vec![vec![String::from("comic-1")]]
    );

    assert_eq!(
        obj_dept.calls("user-list"),
        vec![vec![String::from("user-1")]]
    );
}

#[tokio::test]
async fn nested_repeated_models_load_once_per_object_marker() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    obj_dept.omit("cover-list", "comic-1");

    let assignment_info = assignment_info();

    let assignment_views = assignment_info_views(
        &repo,
        &obj_dept,
        vec![assignment_info.clone(), assignment_info],
    )
    .await
    .unwrap();

    assert_eq!(assignment_views.len(), 2);

    for assignment_view in assignment_views {
        //
        assert_eq!(
            assignment_view
                .user
                .unwrap()
                .avatar_thumbnail_url
                .as_deref(),
            Some("https://obj.test/thumbnail/user-1")
        );

        let comic_view = assignment_view.chapter.unwrap().comic.unwrap();

        assert_eq!(
            comic_view.cover_thumbnail_url.as_deref(),
            Some("https://obj.test/thumbnail/page-1")
        );

        assert_eq!(
            comic_view.team.unwrap().avatar_thumbnail_url.as_deref(),
            Some("https://obj.test/thumbnail/team-1")
        );
    }

    assert_eq!(obj_dept.calls("cover-urls"), vec![Vec::<String>::new()]);

    for (operation, id) in [
        ("cover-list", "comic-1"),
        ("team-list", "team-1"),
        ("team-urls", "team-1"),
        ("user-list", "user-1"),
        ("user-urls", "user-1"),
        ("page-list", "page-1"),
        ("page-urls", "page-1"),
    ] {
        //
        assert_eq!(obj_dept.calls(operation), vec![vec![id.to_owned()]]);
    }
}

#[tokio::test]
async fn empty_lists_perform_no_object_or_repository_operations() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    let chapters = chapter_info_views(&repo, &obj_dept, Vec::new())
        .await
        .unwrap();

    let assignments = assignment_info_views(&repo, &obj_dept, Vec::new())
        .await
        .unwrap();

    assert!(chapters.is_empty());

    assert!(assignments.is_empty());

    assert!(obj_dept.is_empty());

    assert!(repo.pinned_chapter_calls().is_empty());

    assert!(repo.first_page_calls().is_empty());
}

#[tokio::test]
async fn partial_assignment_skips_absent_markers() {
    //
    let obj_dept = TestObjDept::default();

    let mut assignment_info = assignment_info();

    assignment_info.chapter = None;

    let assignment_view = assignment_info_view(&obj_dept, assignment_info)
        .await
        .unwrap();

    assert!(assignment_view.chapter.is_none());

    assert_eq!(obj_dept.calls("user-list").len(), 1);

    assert_eq!(obj_dept.calls("user-urls").len(), 1);

    for operation in [
        "cover-list",
        "cover-urls",
        "team-list",
        "team-urls",
        "page-list",
        "page-urls",
    ] {
        assert!(obj_dept.calls(operation).is_empty());
    }
}

#[tokio::test]
async fn assignment_list_deduplicates_and_sorts_each_batch() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    let mut assignment_infos = Vec::new();

    for user_id in ["user-z", "user-a", "user-z"] {
        //
        let mut assignment_info = assignment_info();

        assignment_info.chapter = None;

        assignment_info.user_id = user_id.into();

        assignment_info.user.as_mut().unwrap().id = user_id.into();

        assignment_infos.push(assignment_info);
    }

    let assignment_views =
        assignment_info_views(&repo, &obj_dept, assignment_infos)
            .await
            .unwrap();

    assert_eq!(
        assignment_views
            .iter()
            .map(|view| view.user_id.as_str())
            .collect::<Vec<_>>(),
        vec!["user-z", "user-a", "user-z"]
    );

    let expected_ids = vec![String::from("user-a"), String::from("user-z")];

    assert_eq!(obj_dept.calls("user-list"), vec![expected_ids.clone()]);

    assert_eq!(obj_dept.calls("user-urls"), vec![expected_ids]);

    assert!(repo.pinned_chapter_calls().is_empty());
}

#[tokio::test]
async fn metadata_error_is_propagated_without_url_generation() {
    //
    let obj_dept = TestObjDept::default();

    obj_dept.fail("user-list");

    let mut assignment_info = assignment_info();

    assignment_info.chapter = None;

    let result = assignment_info_view(&obj_dept, assignment_info).await;

    assert!(result.is_err());

    assert!(obj_dept.calls("user-urls").is_empty());
}

#[tokio::test]
async fn url_error_is_propagated_after_metadata_load() {
    //
    let obj_dept = TestObjDept::default();

    obj_dept.fail("user-urls");

    let mut assignment_info = assignment_info();

    assignment_info.chapter = None;

    let result = assignment_info_view(&obj_dept, assignment_info).await;

    assert!(result.is_err());

    assert_eq!(obj_dept.calls("user-list").len(), 1);

    assert_eq!(obj_dept.calls("user-urls").len(), 1);
}

#[tokio::test]
async fn single_assignment_does_not_load_cover_fallback() {
    //
    let obj_dept = TestObjDept::default();

    obj_dept.omit("cover-list", "comic-1");

    let assignment_info = assignment_info();

    let assignment_view = assignment_info_view(&obj_dept, assignment_info)
        .await
        .unwrap();

    let comic_view = assignment_view.chapter.unwrap().comic.unwrap();

    assert!(comic_view.cover_url.is_none());

    assert!(comic_view.cover_thumbnail_url.is_none());

    assert!(obj_dept.calls("page-list").is_empty());

    assert!(obj_dept.calls("page-urls").is_empty());
}
