use super::*;

#[tokio::test]
async fn comic_sweep_claims_at_most_sixty_four_childless_rows() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 65));

    for index in 0..65 {
        let comic_id = format!("comic-{index:02}");

        mock.seed_comic(comic(&comic_id, "workset-1", index));

        seed_comic_cover(&mock, &comic_id);

        mock.state
            .lock()
            .unwrap()
            .deleted_comic_ids
            .insert(comic_id);
    }

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic)
            .await
            .unwrap()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);
    assert_eq!(snapshot.deleted_comic_ids.len(), 1);
    assert_eq!(snapshot.obj_tasks.len(), 64);

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic)
            .await
            .unwrap()
    );

    assert!(mock.snapshot().comics.is_empty());
}

#[tokio::test]
async fn comic_sweep_skips_parent_with_physical_chapter() {
    let mock = Mock::new();

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 1));
    mock.seed_comic(comic("comic-1", "workset-1", 0));
    mock.seed_chapter(chapter("chapter-1", "comic-1"));

    mock.state
        .lock()
        .unwrap()
        .deleted_comic_ids
        .insert("comic-1".into());

    assert!(
        !sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic)
            .await
            .unwrap()
    );

    assert_eq!(mock.snapshot().comics.len(), 1);
}

#[tokio::test]
async fn comic_sweep_rolls_back_the_whole_claim_on_object_failure() {
    let mock = Mock::new().with_obj_delete_failure();

    mock.seed_workset(workset_with_comic_count("workset-1", "team-1", 0, 1));
    mock.seed_comic(comic("comic-1", "workset-1", 0));
    seed_comic_cover(&mock, "comic-1");

    mock.state
        .lock()
        .unwrap()
        .deleted_comic_ids
        .insert("comic-1".into());

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Comic)
            .await
            .is_err()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.comics.len(), 1);
    assert!(snapshot.deleted_comic_ids.contains("comic-1"));
    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.objs["comic_cover"].contains_key("comic-1"));
}

#[tokio::test]
async fn workset_sweep_claims_at_most_sixty_four_rows() {
    let mock = Mock::new();

    for index in 0..65 {
        let workset_id = format!("workset-{index:02}");

        mock.seed_workset(workset(&workset_id, "team-1", index));

        mock.state
            .lock()
            .unwrap()
            .deleted_workset_ids
            .insert(workset_id);
    }

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Workset)
            .await
            .unwrap()
    );

    assert_eq!(mock.snapshot().worksets.len(), 1);

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Workset)
            .await
            .unwrap()
    );

    assert!(mock.snapshot().worksets.is_empty());
}

#[tokio::test]
async fn chapter_and_team_sweeps_claim_one_row() {
    let mock = Mock::new();

    mock.seed_team(team("team-1", "Team 1", "Desc"));
    mock.seed_team(team("team-2", "Team 2", "Desc"));
    mock.seed_workset(workset("workset-1", "team-1", 0));
    mock.seed_comic(comic("comic-1", "workset-1", 0));
    mock.seed_chapter(chapter("chapter-1", "comic-1"));
    mock.seed_chapter(chapter("chapter-2", "comic-1"));

    {
        let mut state = mock.state.lock().unwrap();

        state.deleted_chapter_ids.insert("chapter-1".into());
        state.deleted_chapter_ids.insert("chapter-2".into());
        state.deleted_team_ids.insert("team-1".into());
        state.deleted_team_ids.insert("team-2".into());
    }

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Chapter)
            .await
            .unwrap()
    );

    assert_eq!(mock.snapshot().chapters.len(), 1);

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Team)
            .await
            .unwrap()
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams.len(), 1);
    assert_eq!(snapshot.deleted_team_ids.len(), 1);
}
