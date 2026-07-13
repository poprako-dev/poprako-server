// comment_roundtrip_reads_test_database_url(CreateComment, ListCommentInfos)(positive): comment repo creates and lists included users in the local test database.

use super::*;

use poprako_orchestra::Nucl as _;

use crate::model::comment::{CommentEntry,CommentListSpec};
use crate::part::repo::oper::comment::{CreateComment, ListCommentInfos};
use crate::part_impl::drive::rdb_impl::RdbDrive;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::value::comment::CommentInclOpt;

const PREFIX: &str = "rdb-test-comment-domain-";

#[tokio::test]
async fn comment_roundtrip_reads_test_database_url() {
    //
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let comment_entry = CommentEntry {
        id: format!("{}comment", PREFIX),
        team_id: team_fixture.team_entry.id.clone(),
        user_id: team_fixture.user_entry.id.clone(),
        content: "comment".into(),
    };

    drive
        .coord(async |context| {
            //
            repo.step(
                context,
                &CreateComment {
                    entry: &comment_entry,
                },
            )
            .await?;

            Ok::<(), RegularError>(())
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
