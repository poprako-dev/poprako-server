use super::*;

use poprako_obj_dept::model::task::ObjTask;

use crate::data::instr::team::AllocTeamAvatarInstr;
use crate::test_util::IMAGE_CONFIG;
use crate::usecase::subtree_delete::sweep;
use crate::value::image::{ImageExt, ImageHash};
use crate::value::subtree_delete::SubtreeSweepLevel;

fn alloc_instr(hash_byte: u8, ext: ImageExt) -> AllocTeamAvatarInstr {
    AllocTeamAvatarInstr {
        image_hash: ImageHash::new([hash_byte; 32]),
        new_byte_len: 4096,
        ext,
    }
}

fn seed_alloc_scope(mock: &Mock) {
    mock.seed_team(team("team-1", "Team", "Desc"));

    mock.seed_member(member("member-1", "user-1", "team-1"));
}

fn mark_seeded_avatar_available(mock: &Mock) {
    mock.state
        .lock()
        .unwrap()
        .objs
        .get_mut("team_avatar")
        .unwrap()
        .get_mut("team-1")
        .unwrap()
        .meta
        .as_mut()
        .unwrap()
        .is_avail = true;
}

#[tokio::test]
async fn alloc_avatar_creates_generation_check_and_put_url() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .unwrap();

    let slot = allocated.slot.unwrap();

    assert_eq!(slot.image_ver, 1);
    assert_eq!(
        slot.put_url,
        "https://obj.test/write/team_avatar/team-1-1.png",
    );

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.objs["team_avatar"]["team-1"].version, 1);
    assert!(matches!(snapshot.obj_tasks[0].1, ObjTask::Check { .. }));
}

#[tokio::test]
async fn alloc_avatar_replacement_deletes_old_and_checks_new_generation() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);
    seed_team_avatar(&mock, 1);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        alloc_instr(2, ImageExt::Jpg),
    )
    .await
    .unwrap();

    assert_eq!(allocated.slot.unwrap().image_ver, 2);

    let snapshot = mock.snapshot();

    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.ver == 1)
    }));
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Check { key, .. } if key.ver == 2)
    }));
}

#[tokio::test]
async fn alloc_avatar_rolls_back_when_team_is_missing() {
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));

    let err = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().objs.is_empty());
    assert!(mock.snapshot().obj_tasks.is_empty());
}

#[tokio::test]
async fn alloc_avatar_rejects_byte_length_above_team_limit() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);

    let image_config = crate::config::image::ImageConfig {
        team_avatar_limit: 1,
        ..IMAGE_CONFIG
    };
    let mut instr = alloc_instr(1, ImageExt::Png);
    instr.new_byte_len = 1024 * 1024 + 1;

    let err = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &image_config),
        token("user-1"),
        "team-1".into(),
        instr,
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert!(mock.snapshot().objs.is_empty());
}

#[tokio::test]
async fn alloc_avatar_returns_no_slot_for_same_available_content() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);
    seed_team_avatar(&mock, 1);
    mark_seeded_avatar_available(&mock);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        alloc_instr(0, ImageExt::Png),
    )
    .await
    .unwrap();

    assert!(allocated.slot.is_none());
    assert!(mock.snapshot().obj_tasks.is_empty());
}

#[tokio::test]
async fn mark_avatar_uploaded_accepts_repeated_current_generation() {
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));
    seed_team_avatar(&mock, 2);

    mark_team_avatar(&mock, 2).await.unwrap();
    mark_team_avatar(&mock, 2).await.unwrap();

    assert!(
        mock.snapshot().objs["team_avatar"]["team-1"]
            .meta
            .as_ref()
            .unwrap()
            .is_avail
    );
}

#[tokio::test]
async fn mark_avatar_uploaded_rejects_old_allocation_replay() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);
    seed_team_avatar(&mock, 1);

    let allocated = alloc_avatar::<_, MockContext, _, _>(
        (&mock, &mock, &mock, &IMAGE_CONFIG),
        token("user-1"),
        "team-1".into(),
        alloc_instr(1, ImageExt::Png),
    )
    .await
    .unwrap();

    assert_eq!(allocated.slot.unwrap().image_ver, 2);

    let err = mark_team_avatar(&mock, 1).await.err().unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);
    assert_eq!(mock.snapshot().objs["team_avatar"]["team-1"].version, 2);
}

#[tokio::test]
async fn delete_marks_team_without_eager_avatar_delete() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);
    seed_team_avatar(&mock, 2);
    mark_seeded_avatar_available(&mock);

    delete::delete::<_, MockContext, _>(
        (&mock, &mock),
        token("user-1"),
        "team-1".into(),
    )
    .await
    .unwrap();

    let snapshot = mock.snapshot();

    assert_eq!(snapshot.teams.len(), 1);
    assert!(snapshot.deleted_team_ids.contains("team-1"));
    assert!(snapshot.objs["team_avatar"].contains_key("team-1"));
    assert!(snapshot.obj_tasks.is_empty());
}

#[tokio::test]
async fn sweep_eligible_team_enqueues_exact_avatar_delete() {
    let mock = Mock::new();

    seed_alloc_scope(&mock);
    seed_team_avatar(&mock, 2);

    delete::delete::<_, MockContext, _>(
        (&mock, &mock),
        token("user-1"),
        "team-1".into(),
    )
    .await
    .unwrap();

    assert!(
        sweep((&mock, &mock, &mock), SubtreeSweepLevel::Team)
            .await
            .unwrap()
    );

    let snapshot = mock.snapshot();

    assert!(snapshot.teams.is_empty());
    assert!(snapshot.deleted_team_ids.is_empty());
    assert!(snapshot.obj_tasks.iter().any(|(_, task)| {
        matches!(task, ObjTask::Delete { key } if key.id == "team-1" && key.ver == 2)
    }));
}

#[tokio::test]
async fn delete_missing_team_rolls_back_avatar_debt() {
    let mock = Mock::new();

    mock.seed_member(member("member-1", "user-1", "team-1"));
    seed_team_avatar(&mock, 2);

    let err = delete::delete::<_, MockContext, _>(
        (&mock, &mock),
        token("user-1"),
        "team-1".into(),
    )
    .await
    .err()
    .unwrap();

    assert_expected_variant(err, ExpectedVariant::Args);

    let snapshot = mock.snapshot();

    assert!(snapshot.obj_tasks.is_empty());
    assert!(snapshot.objs["team_avatar"].contains_key("team-1"));
}
