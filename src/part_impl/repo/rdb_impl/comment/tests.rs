// comment_roundtrip_uses_testcontainer(CreateComment, ListCommentInfos)(positive): comment repo creates and lists included users in an isolated PostgreSQL container.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::read::spec::comment::CommentListSpec;
use crate::model::write::comment::CommentEntry;
use crate::part::nucl::RepeatableRead;
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::nucl::rdb_impl::RdbNucl;
use crate::part_impl::repo::HybRepo;
use crate::part_impl::repo::rdb_impl::test_shared;
use crate::result::BaseError;
use crate::shared::RdbCore;
use crate::value::comment::CommentInclOpt;

const PREFIX: &str = "rdb-test-comment-domain-";

/// Verifies comment roundtrip via testcontainers.
/// Verifies comment roundtrip via testcontainers.
pub async fn comment_roundtrip_uses_testcontainer(shared: RdbCore) {
    //
    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = HybRepo::new(shared.clone());

    let nucl = RdbNucl::<RepeatableRead>::new(shared.clone());

    let comment_entry = CommentEntry {
        id: format!("{}comment", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        user_id: team_fixture.user_entry.id.clone(),
        content: "comment".into(),
    };

    nucl.coord(async |context| {
        //
        repo.step(
            context,
            &CreateComment {
                entry: &comment_entry,
            },
        )
        .await?;

        Ok::<(), BaseError>(())
    })
    .await
    .ok()
    .unwrap();

    let comment_list_spec = CommentListSpec {
        team_id: team_fixture.team_entry.id.clone(),
        incl_opt: vec![CommentInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let comment_infos = repo
        .run(&ListCommentInfos {
            spec: &comment_list_spec,
        })
        .await
        .ok()
        .unwrap();

    assert_eq!(comment_infos.len(), 1);

    assert_eq!(
        comment_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_entry.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
