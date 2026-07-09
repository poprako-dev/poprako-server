// user_roundtrip_reads_test_database_url(UserStep)(positive): user repo persists and reloads a user from the local test database.

use crate::part::repo::step::user::UserStep;
use crate::part::shared::execute::Execute;
use crate::part_impl::repo::rdb_impl::{RdbRepo, test_shared};

const PREFIX: &str = "rdb-test-user-domain-";

#[tokio::test]
async fn user_roundtrip_reads_test_database_url() {
    let shared = test_shared::shared().await;

    test_shared::reset(&shared, PREFIX).await;

    let user_fixture = test_shared::seed_user(&shared, PREFIX).await;

    let repo = RdbRepo::new(shared.clone());

    let user_info = Execute::execute(
        &repo,
        &UserStep::get_info_by_id(&user_fixture.user_form.id),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(user_info.id, user_fixture.user_form.id);

    let user_credential = Execute::execute(
        &repo,
        &UserStep::get_credential_by_qid(&user_fixture.user_form.qid),
    )
    .await
    .ok()
    .unwrap();

    assert_eq!(user_credential.user_id, user_fixture.user_form.id);

    let found_user_info = Execute::execute(
        &repo,
        &UserStep::find_info_by_qid(&user_fixture.user_form.qid),
    )
    .await
    .ok()
    .unwrap()
    .unwrap();

    assert_eq!(found_user_info.id, user_fixture.user_form.id);

    test_shared::cleanup(&shared, PREFIX).await.ok().unwrap();

    test_shared::assert_no_leftovers(&shared, PREFIX)
        .await
        .ok()
        .unwrap();
}
