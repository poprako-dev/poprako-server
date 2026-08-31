use super::{
    Mock, RoleMask, Stage, UpdateChapterStageInstr, assignment, chapter,
    seed_scope, token, update_stage,
};

use crate::value::chapter::stage::{StageOper, StagePhase};
use crate::value::role::RoleField;

#[tokio::test]
async fn update_stage_admin_reverts_without_role_holder() {
    //
    let mock = Mock::new();

    seed_scope(&mock, "user-1", RoleMask::from(RoleField::ADMIN));

    let mut chapter_info = chapter("chapter-1", "comic-1", 1, false);

    chapter_info.stages = chapter_info
        .stages
        .try_set_phase(Stage::Translate, StagePhase::Active)
        .ok()
        .unwrap();

    mock.seed_chapter(chapter_info);

    mock.seed_assignment(assignment(
        "chapter-1",
        "user-1",
        RoleMask::from(RoleField::ADMIN),
    ));

    update_stage(
        (&mock, &mock, &mock, &mock),
        token("user-1"),
        UpdateChapterStageInstr {
            id: "chapter-1".into(),
            stage: Stage::Translate.into(),
            oper: StageOper::Revert.into(),
        },
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(
        mock.snapshot().chapters[0]
            .stages
            .get_phase(Stage::Translate),
        StagePhase::Pending
    );
}
