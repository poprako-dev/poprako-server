// comment_roundtrip_reads_test_database_url(CommentStep)(positive): comment repo creates and lists included users in the local test database.

use poprako_transactional::advance::Advance;
use poprako_transactional::drive::Drive;

use crate::model::comment::{CommentForm, CommentListSpec};
use crate::part::repo::step::comment::CommentStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::drive_rdb::RdbDrive;
use crate::part_impl::repo_rdb::{RdbRepo, test_shared};
use crate::result::RegularError;
use crate::util::DeriveTransactional as _;
use crate::value::comment::CommentInclOpt;

const PREFIX: &str = "rdb-test-comment-domain-";

#[tokio::test]
async fn comment_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let team_fixture = test_shared::seed_user_and_team(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let drive = RdbDrive::new(shared.clone());

    let transactional_repo = repo.derive_transactional().await;

    let comment_form = CommentForm {
        id: format!("{}comment", PREFIX),
        team_id: team_fixture.team_form.id.clone(),
        user_id: team_fixture.user_form.id.clone(),
        content: "comment".into(),
    };

    drive
        .with_context(async |context| {
            Advance::advance(
                &transactional_repo,
                context,
                &CommentStep::create(&comment_form),
            )
            .await?;

            Ok::<(), RegularError>(())
        })
        .await
        .ok()
        .unwrap();

    let comment_list_spec = CommentListSpec {
        team_id: team_fixture.team_form.id.clone(),
        incl_opt: vec![CommentInclOpt::User],
        offset: 0,
        limit: 10,
    };

    let comment_infos = Execute::execute(&repo, &CommentStep::list_infos(&comment_list_spec))
        .await
        .ok()
        .unwrap();

    assert_eq!(comment_infos.len(), 1);
    assert_eq!(
        comment_infos[0].user.as_ref().unwrap().id,
        team_fixture.user_form.id
    );

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
