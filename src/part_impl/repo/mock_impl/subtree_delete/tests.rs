// review_subtree_cleanup(delete_chapter)(positive): physical subtree cleanup removes Chapter-scoped current review details.
#[test]
fn subtree_cleanup_removes_current_review_issues() {
    use super::sweep;
    use crate::model::read::proj::issue::IssueInfo;
    use crate::model::read::proj::page::PageInfo;
    use crate::part_impl::repo::mock_impl::MockState;
    use time::OffsetDateTime;

    let now = OffsetDateTime::now_utc();

    let mut state = MockState::default();

    state.pages.push(PageInfo {
        id: "page-review".into(),
        chapter_id: "chapter-review".into(),
        index: 0,
        total_unit_count: 0,
        translated_unit_count: 0,
        proofread_unit_count: 0,
        created_at: now,
        updated_at: now,
    });

    state.page_artworks.push(
        crate::model::read::proj::page_artwork::PageArtworkInfo {
            id: "artwork-review".into(),
            chapter_id: "chapter-review".into(),
            index: 5,
            raw_ident: None,
            created_at: now,
            updated_at: now,
        },
    );

    state.issues.push(IssueInfo {
        id: "issue-review".into(),
        page_artwork_id: "artwork-review".into(),
        index: 0,
        variant: "custom".into(),
        layer_name: None,
        rect: None,
        note: "review".into(),
    });

    sweep::delete_chapter(&mut state, "chapter-review");

    assert!(state.issues.is_empty());

    assert!(state.pages.is_empty());
}
