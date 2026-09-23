//! Comic list rendering shares lookup state across its complete response graph.

use crate::usecase::internal::page::PinnedChapterSnapshot;
use crate::usecase::internal::view::{ComicListData, comic_list_val};

use super::fixture::{TestObjDept, TestRepo, assignment_info};

// mixed_comic_list_reuses_snapshot_and_batches_complete_graph(comic_list_val)(positive): top-level comics, pinned chapters and assignments share deduplicated URLs and preserve aligned order including known-absent chapters.
// partial_snapshot_only_loads_unqueried_comics(comic_list_val)(positive): cover fallback only reads comics outside the existing snapshot range.
// empty_comic_list_performs_no_lookups(comic_list_val)(positive): empty results preserve all aligned vectors without object or repository reads.
// absent_pinned_chapters_skip_first_page_lookup(comic_list_val)(positive): deduplicated known-absent comics require no repeated pin query or empty first-page query.
// distinct_comics_keep_nested_urls_aligned(comic_list_val)(positive): different root, chapter, and assignment orders retain each model's own cover and avatar.

#[tokio::test]
async fn distinct_comics_keep_nested_urls_aligned() {
    //
    let obj_dept = TestObjDept::default();

    let mut assignments = Vec::new();

    for suffix in ["z", "a"] {
        //
        let mut assignment = assignment_info();

        assignment.id = format!("assignment-{suffix}");

        assignment.chapter_id = format!("chapter-{suffix}");

        assignment.user_id = format!("user-{suffix}");

        assignment.user.as_mut().unwrap().id = assignment.user_id.clone();

        let chapter = assignment.chapter.as_mut().unwrap();

        chapter.id = assignment.chapter_id.clone();

        chapter.comic_id = format!("comic-{suffix}");

        chapter.creator_id = format!("creator-{suffix}");

        chapter.creator.as_mut().unwrap().id = chapter.creator_id.clone();

        let comic = chapter.comic.as_mut().unwrap();

        comic.id = chapter.comic_id.clone();

        comic.creator_id = format!("owner-{suffix}");

        comic.creator.as_mut().unwrap().id = comic.creator_id.clone();

        assignments.push(assignment);
    }

    let comics = assignments
        .iter()
        .map(|assignment| {
            assignment.chapter.as_ref().unwrap().comic.clone().unwrap()
        })
        .collect::<Vec<_>>();

    let chapters = assignments
        .iter()
        .rev()
        .map(|assignment| assignment.chapter.clone().unwrap())
        .collect();

    let repo = TestRepo::with_chapters(chapters);

    let pinned_chapters = PinnedChapterSnapshot::load_from_comics(
        &repo,
        &comics,
        0..comics.len(),
    )
    .await
    .unwrap();

    assignments.reverse();

    let data = ComicListData {
        comics,
        pinned_chapters: Some(pinned_chapters),
        assignments,
    };

    let list_val = comic_list_val(&repo, &obj_dept, data).await.unwrap();

    for (position, suffix) in ["z", "a"].into_iter().enumerate() {
        //
        let expected_cover = format!("https://obj.test/comic-{suffix}");

        let chapter = list_val.pinned_chapters[position].as_ref().unwrap();

        let assignment = &list_val.pinned_chapter_assignments[position][0];

        assert_eq!(list_val.comics[position].id, format!("comic-{suffix}"));

        assert_eq!(
            list_val.comics[position].cover_url.as_deref(),
            Some(expected_cover.as_str())
        );

        assert_eq!(chapter.id, format!("chapter-{suffix}"));

        assert_eq!(
            chapter.comic.as_ref().unwrap().cover_url.as_deref(),
            Some(expected_cover.as_str())
        );

        assert_eq!(assignment.id, format!("assignment-{suffix}"));

        assert_eq!(
            assignment
                .chapter
                .as_ref()
                .unwrap()
                .comic
                .as_ref()
                .unwrap()
                .cover_url
                .as_deref(),
            Some(expected_cover.as_str())
        );

        let expected_avatar = format!("https://obj.test/user-{suffix}");

        assert_eq!(
            assignment.user.as_ref().unwrap().avatar_url.as_deref(),
            Some(expected_avatar.as_str())
        );
    }
}

#[tokio::test]
async fn mixed_comic_list_reuses_snapshot_and_batches_complete_graph() {
    //
    let obj_dept = TestObjDept::default();

    obj_dept.omit("cover-list", "comic-1");

    obj_dept.omit("cover-list", "comic-z");

    let mut first_assignment = assignment_info();

    first_assignment.id = "assignment-z".into();

    first_assignment.user_id = "user-a".into();

    first_assignment.user.as_mut().unwrap().id = "user-a".into();

    let pinned_chapter = first_assignment.chapter.as_mut().unwrap();

    pinned_chapter.creator_id = "user-z".into();

    pinned_chapter.creator.as_mut().unwrap().id = "user-z".into();

    let comic_info = pinned_chapter.comic.clone().unwrap();

    let repo = TestRepo::with_chapters(vec![pinned_chapter.clone()]);

    let mut missing_comic = comic_info.clone();

    missing_comic.id = "comic-z".into();

    let mut second_assignment = first_assignment.clone();

    second_assignment.id = "assignment-a".into();

    let comics = vec![missing_comic, comic_info];

    let pinned_snapshot = PinnedChapterSnapshot::load_from_comics(
        &repo,
        &comics,
        0..comics.len(),
    )
    .await
    .unwrap();

    let data = ComicListData {
        comics,
        pinned_chapters: Some(pinned_snapshot),
        assignments: vec![first_assignment, second_assignment],
    };

    let list_val = comic_list_val(&repo, &obj_dept, data).await.unwrap();

    assert_eq!(
        list_val
            .comics
            .iter()
            .map(|comic| comic.id.as_str())
            .collect::<Vec<_>>(),
        vec!["comic-z", "comic-1"]
    );

    assert_eq!(list_val.pinned_chapters.len(), 2);

    assert_eq!(list_val.pinned_chapter_assignments.len(), 2);

    assert!(list_val.pinned_chapters[0].is_none());

    assert!(list_val.pinned_chapter_assignments[0].is_empty());

    assert!(list_val.comics[0].cover_url.is_none());

    assert_eq!(
        list_val.comics[1].cover_url.as_deref(),
        Some("https://obj.test/page-1")
    );

    let pinned_view = list_val.pinned_chapters[1].as_ref().unwrap();

    assert_eq!(pinned_view.id, "chapter-1");

    assert_eq!(
        pinned_view.creator.as_ref().unwrap().avatar_url.as_deref(),
        Some("https://obj.test/user-z")
    );

    assert_eq!(
        pinned_view.comic.as_ref().unwrap().cover_url.as_deref(),
        Some("https://obj.test/page-1")
    );

    assert_eq!(
        list_val.pinned_chapter_assignments[1]
            .iter()
            .map(|assignment| assignment.id.as_str())
            .collect::<Vec<_>>(),
        vec!["assignment-z", "assignment-a"]
    );

    for assignment in &list_val.pinned_chapter_assignments[1] {
        //
        assert_eq!(
            assignment.user.as_ref().unwrap().avatar_url.as_deref(),
            Some("https://obj.test/user-a")
        );

        assert_eq!(
            assignment
                .chapter
                .as_ref()
                .unwrap()
                .comic
                .as_ref()
                .unwrap()
                .cover_url
                .as_deref(),
            Some("https://obj.test/page-1")
        );
    }

    for operation in ["cover-list", "cover-urls"] {
        //
        let expected = match operation {
            "cover-list" => {
                vec![String::from("comic-1"), String::from("comic-z")]
            }
            _ => Vec::new(),
        };

        assert_eq!(obj_dept.calls(operation), vec![expected]);
    }

    for operation in ["user-list", "user-urls"] {
        assert_eq!(
            obj_dept.calls(operation),
            vec![vec![
                String::from("user-1"),
                String::from("user-a"),
                String::from("user-z")
            ]]
        );
    }

    for operation in ["team-list", "team-urls"] {
        assert_eq!(
            obj_dept.calls(operation),
            vec![vec![String::from("team-1")]]
        );
    }

    for operation in ["page-list", "page-urls"] {
        assert_eq!(
            obj_dept.calls(operation),
            vec![vec![String::from("page-1")]]
        );
    }

    assert_eq!(
        repo.pinned_chapter_calls(),
        vec![vec![String::from("comic-1"), String::from("comic-z")]]
    );

    assert_eq!(
        repo.first_page_calls(),
        vec![vec![String::from("chapter-1")]]
    );
}

#[tokio::test]
async fn partial_snapshot_only_loads_unqueried_comics() {
    //
    let obj_dept = TestObjDept::default();

    obj_dept.omit("cover-list", "comic-1");

    obj_dept.omit("cover-list", "comic-z");

    let repo = TestRepo::default();

    let comic_info = assignment_info().chapter.unwrap().comic.unwrap();

    let mut missing_comic = comic_info.clone();

    missing_comic.id = "comic-z".into();

    let comics = vec![comic_info, missing_comic];

    let pinned_snapshot =
        PinnedChapterSnapshot::load_from_comics(&repo, &comics, 1..2)
            .await
            .unwrap();

    let data = ComicListData {
        comics,
        pinned_chapters: Some(pinned_snapshot),
        assignments: Vec::new(),
    };

    let list_val = comic_list_val(&repo, &obj_dept, data).await.unwrap();

    assert_eq!(
        repo.pinned_chapter_calls(),
        vec![vec![String::from("comic-z")], vec![String::from("comic-1")]]
    );

    assert_eq!(
        list_val.comics[0].cover_url.as_deref(),
        Some("https://obj.test/page-1")
    );

    assert!(list_val.comics[1].cover_url.is_none());

    assert!(list_val.pinned_chapters.iter().all(Option::is_none));

    assert!(
        list_val
            .pinned_chapter_assignments
            .iter()
            .all(Vec::is_empty)
    );
}

#[tokio::test]
async fn empty_comic_list_performs_no_lookups() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::default();

    let data = ComicListData {
        comics: Vec::new(),
        pinned_chapters: None,
        assignments: Vec::new(),
    };

    let list_val = comic_list_val(&repo, &obj_dept, data).await.unwrap();

    assert!(list_val.comics.is_empty());

    assert!(list_val.pinned_chapters.is_empty());

    assert!(list_val.pinned_chapter_assignments.is_empty());

    assert!(obj_dept.is_empty());

    assert!(repo.pinned_chapter_calls().is_empty());

    assert!(repo.first_page_calls().is_empty());
}

#[tokio::test]
async fn absent_pinned_chapters_skip_first_page_lookup() {
    //
    let obj_dept = TestObjDept::default();

    let repo = TestRepo::with_chapters(Vec::new());

    let comic_info = assignment_info().chapter.unwrap().comic.unwrap();

    obj_dept.omit("cover-list", &comic_info.id);

    let comics = vec![comic_info.clone(), comic_info];

    let pinned_snapshot = PinnedChapterSnapshot::load_from_comics(
        &repo,
        &comics,
        0..comics.len(),
    )
    .await
    .unwrap();

    let data = ComicListData {
        comics,
        pinned_chapters: Some(pinned_snapshot),
        assignments: Vec::new(),
    };

    let list_val = comic_list_val(&repo, &obj_dept, data).await.unwrap();

    assert_eq!(list_val.comics.len(), 2);

    assert!(list_val.pinned_chapters.iter().all(Option::is_none));

    assert!(list_val.comics[0].cover_url.is_none());

    assert!(list_val.pinned_chapters[0].is_none());

    assert_eq!(repo.pinned_chapter_calls(), vec![vec!["comic-1"]]);

    assert!(repo.first_page_calls().is_empty());

    assert!(obj_dept.calls("page-list").is_empty());
}
